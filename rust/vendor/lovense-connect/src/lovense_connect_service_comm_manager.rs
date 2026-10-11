// Buttplug Rust Source Code File - See https://buttplug.io for more info.
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
// Licensed under the BSD 3-Clause license; see LICENSE in this directory.
// Lovense Connect maintenance changes for this Intiface Central fork.
use super::lovense_connect_service_hardware::LovenseServiceHardwareConnector;
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager, HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent, TimedRetryCommunicationManager,
  TimedRetryCommunicationManagerImpl,
};
use dashmap::DashSet;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Deserializer};
use serde_aux::prelude::*;
use std::{
  collections::HashMap,
  net::Ipv4Addr,
  sync::{LazyLock, RwLock},
  time::Duration,
};
use tokio::sync::mpsc::Sender;

static MANUAL_HOSTS: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| RwLock::new(vec![]));
static LOCAL_CLIENT: LazyLock<Client> = LazyLock::new(|| {
  Client::builder()
    .no_proxy()
    .timeout(Duration::from_secs(3))
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .expect("Building local HTTP client")
});

pub fn validate_manual_host(input: &str) -> Result<String, String> {
  let url = Url::parse(input.trim()).map_err(|_| "Invalid Lovense Connect phone address")?;
  let ip: Ipv4Addr = url
    .host_str()
    .unwrap_or_default()
    .parse()
    .map_err(|_| "Lovense Connect requires a local IPv4 address")?;
  let port = url.port_or_known_default().unwrap_or(0);
  // url::Url canonicalizes an explicitly supplied :80 away; it remains a valid HTTP endpoint.
  if url.scheme() != "http"
    || !(ip.is_private() || ip.is_loopback())
    || port == 0
    || !url.username().is_empty()
    || url.password().is_some()
    || url.path() != "/"
    || url.query().is_some()
    || url.fragment().is_some()
  {
    return Err(
      "Use a local HTTP phone address and port, such as http://192.168.1.50:20010".into(),
    );
  }
  Ok(format!("http://{ip}:{port}"))
}

/// Set once while reserving engine startup; each new manager owns a snapshot.
/// No environment mutation or FFI ABI change is required.
pub fn set_manual_hosts(hosts: Vec<String>) -> Result<(), String> {
  let hosts = hosts
    .iter()
    .map(|h| validate_manual_host(h))
    .collect::<Result<Vec<_>, _>>()?;
  *MANUAL_HOSTS
    .write()
    .map_err(|_| "Lovense Connect settings lock is poisoned")? = hosts;
  Ok(())
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct LovenseServiceToyInfo {
  pub id: String,
  pub name: String,
  #[serde(rename = "status", deserialize_with = "deserialize_bool_from_anything")]
  pub connected: bool,
  #[serde(default, deserialize_with = "parse_battery")]
  pub battery: i8,
}
fn parse_battery<'de, D>(d: D) -> Result<i8, D::Error>
where
  D: Deserializer<'de>,
{
  let value = serde_json::Value::deserialize(d)?;
  match value {
    serde_json::Value::Null => Ok(0),
    serde_json::Value::Number(n) => n
      .as_i64()
      .map(|v| v.clamp(0, 100) as i8)
      .ok_or_else(|| serde::de::Error::custom("Battery must be an integer")),
    serde_json::Value::String(s) => s
      .parse::<i64>()
      .map(|v| v.clamp(0, 100) as i8)
      .map_err(serde::de::Error::custom),
    _ => Err(serde::de::Error::custom("Invalid battery")),
  }
}
#[derive(Deserialize, Debug)]
struct LovenseServiceHostInfo {
  #[serde(
    rename = "httpPort",
    deserialize_with = "deserialize_number_from_string"
  )]
  pub http_port: u16,
}
#[derive(Debug)]
pub(super) struct LovenseServiceLocalInfo {
  pub data: HashMap<String, LovenseServiceToyInfo>,
}
type LovenseServiceInfo = HashMap<String, LovenseServiceHostInfo>;

fn decode_json_field(value: serde_json::Value) -> Result<serde_json::Value, String> {
  if let serde_json::Value::String(s) = value {
    serde_json::from_str(&s).map_err(|e| e.to_string())
  } else {
    Ok(value)
  }
}
pub(super) fn check_reply(text: &str) -> Result<serde_json::Value, String> {
  let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
  let code = value
    .get("code")
    .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()));
  if !matches!(code, Some(200 | 0)) {
    return Err(format!(
      "Lovense Connect rejected the request: {}",
      value.get("code").unwrap_or(&serde_json::Value::Null)
    ));
  }
  Ok(value)
}
fn parse_local_info(text: &str) -> Result<LovenseServiceLocalInfo, String> {
  let value = check_reply(text)?;
  let mut data = decode_json_field(value.get("data").cloned().ok_or("Missing toy data")?)?;
  if let Some(toys) = data.get("toys") {
    data = decode_json_field(toys.clone())?;
  }
  let entries = data.as_object().ok_or("Toy data must be an object")?;
  let mut toys = HashMap::new();
  for (id, value) in entries {
    let mut value = value.clone();
    if value.get("id").is_none() {
      if let Some(object) = value.as_object_mut() {
        object.insert("id".into(), id.clone().into());
      }
    }
    match serde_json::from_value::<LovenseServiceToyInfo>(value) {
      Ok(mut toy) => {
        // Lush 4 is the existing single-motor Lush profile in Buttplug.
        if toy.name.eq_ignore_ascii_case("lush")
          || toy.name.eq_ignore_ascii_case("lush 4")
          || toy.name.eq_ignore_ascii_case("lush4")
        {
          toy.name = "Lush".into();
        }
        toys.insert(id.clone(), toy);
      }
      // One malformed device must not make a valid Lush disappear.
      Err(err) => warn!("Ignoring malformed Lovense Connect toy: {err}"),
    }
  }
  Ok(LovenseServiceLocalInfo { data: toys })
}

pub(super) async fn get_local_info(host: &str) -> Option<LovenseServiceLocalInfo> {
  match LOCAL_CLIENT.get(format!("{host}/GetToys")).send().await {
    Ok(res) if res.status() == StatusCode::OK => match res.text().await {
      Ok(text) => match parse_local_info(&text) {
        Ok(info) => Some(info),
        Err(e) => {
          warn!("Invalid Lovense Connect reply: {e}");
          None
        }
      },
      Err(e) => {
        warn!("Reading Lovense Connect reply: {e}");
        None
      }
    },
    Ok(res) => {
      warn!("Lovense Connect HTTP status: {}", res.status());
      None
    }
    Err(err) => {
      warn!("Could not reach Lovense Connect phone: {err}");
      None
    }
  }
}

pub(super) async fn send_local_command(url: Url) -> Result<(), ButtplugDeviceError> {
  let response = LOCAL_CLIENT
    .get(url)
    .send()
    .await
    .map_err(|err| ButtplugDeviceError::UnhandledCommand(err.to_string()))?;
  if response.status() != StatusCode::OK {
    return Err(ButtplugDeviceError::UnhandledCommand(format!(
      "Lovense Connect HTTP {}",
      response.status()
    )));
  }
  let text = response
    .text()
    .await
    .map_err(|err| ButtplugDeviceError::UnhandledCommand(err.to_string()))?;
  check_reply(&text).map_err(ButtplugDeviceError::UnhandledCommand)?;
  Ok(())
}

#[derive(Default, Clone)]
pub struct LovenseConnectServiceCommunicationManagerBuilder {}
impl HardwareCommunicationManagerBuilder for LovenseConnectServiceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(TimedRetryCommunicationManager::new(
      LovenseConnectServiceCommunicationManager::new(sender),
    ))
  }
}
pub struct LovenseConnectServiceCommunicationManager {
  sender: Sender<HardwareCommunicationManagerEvent>,
  known_hosts: DashSet<String>,
  manual_hosts: Vec<String>,
}
impl LovenseConnectServiceCommunicationManager {
  fn new(sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    let hosts = MANUAL_HOSTS.read().map(|h| h.clone()).unwrap_or_default();
    Self::with_hosts(sender, hosts)
  }
  fn with_hosts(
    sender: Sender<HardwareCommunicationManagerEvent>,
    manual_hosts: Vec<String>,
  ) -> Self {
    Self {
      sender,
      known_hosts: DashSet::new(),
      manual_hosts,
    }
  }
  async fn lovense_local_service_check(&self) {
    let hosts: Vec<String> = self.known_hosts.iter().map(|h| h.to_string()).collect();
    for host in hosts {
      if let Some(info) = get_local_info(&host).await {
        for toy in info.data.values().filter(|t| t.connected) {
          let creator = Box::new(LovenseServiceHardwareConnector::new(&host, toy));
          if self
            .sender
            .send(HardwareCommunicationManagerEvent::DeviceFound {
              name: toy.name.clone(),
              address: toy.id.clone(),
              creator,
            })
            .await
            .is_err()
          {
            return;
          }
        }
      } else {
        self.known_hosts.remove(&host);
      }
    }
  }
}
fn discovery_hosts(text: &str) -> Result<Vec<String>, String> {
  let info: LovenseServiceInfo = serde_json::from_str(text).map_err(|e| e.to_string())?;
  Ok(
    info
      .iter()
      .filter_map(|(domain, host)| {
        let ip = domain.split('.').next()?.replace('-', ".");
        validate_manual_host(&format!("http://{ip}:{}", host.http_port)).ok()
      })
      .collect(),
  )
}
#[async_trait]
impl TimedRetryCommunicationManagerImpl for LovenseConnectServiceCommunicationManager {
  fn name(&self) -> &'static str {
    "LovenseServiceDeviceCommManager"
  }
  fn rescan_wait_duration(&self) -> Duration {
    Duration::from_secs(10)
  }
  async fn scan(&self) -> Result<(), ButtplugDeviceError> {
    if !self.manual_hosts.is_empty() {
      // Retry configured hosts on every scan, including after Wi-Fi/app interruption.
      for host in &self.manual_hosts {
        self.known_hosts.insert(host.clone());
      }
    } else if self.known_hosts.is_empty() {
      let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| ButtplugDeviceError::UnhandledCommand(e.to_string()))?;
      match client
        .get("https://api.lovense.com/api/lan/getToys")
        .send()
        .await
      {
        Ok(res) if res.status() == StatusCode::OK => match res.text().await {
          Ok(text) => match discovery_hosts(&text) {
            Ok(hosts) => {
              for host in hosts {
                self.known_hosts.insert(host);
              }
            }
            Err(err) => warn!(
              "Lovense cloud discovery failed: {err}. Configure a manual phone address in Settings."
            ),
          },
          Err(err) => warn!("Reading Lovense cloud discovery: {err}"),
        },
        Ok(res) => warn!(
          "Lovense cloud discovery HTTP {}. Configure a manual phone address in Settings.",
          res.status()
        ),
        Err(err) => warn!(
          "Lovense cloud discovery unavailable: {err}. Configure a manual phone address in Settings."
        ),
      }
    }
    self.lovense_local_service_check().await;
    Ok(())
  }
  fn can_scan(&self) -> bool {
    true
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  fn mock_phone(
    replies: Vec<(u16, &'static str)>,
  ) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let task = std::thread::spawn(move || {
      let mut requests = vec![];
      for (status, body) in replies {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut stream = loop {
          match listener.accept() {
            Ok((s, _)) => break s,
            Err(e)
              if e.kind() == std::io::ErrorKind::WouldBlock
                && std::time::Instant::now() < deadline =>
            {
              std::thread::sleep(Duration::from_millis(5))
            }
            other => panic!("Mock phone timed out: {other:?}"),
          }
        };
        stream
          .set_read_timeout(Some(Duration::from_secs(3)))
          .unwrap();
        let mut data = vec![];
        loop {
          let mut buf = [0u8; 1024];
          let n = stream.read(&mut buf).unwrap();
          if n == 0 {
            break;
          }
          data.extend_from_slice(&buf[..n]);
          if data.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
          }
        }
        requests.push(
          String::from_utf8(data)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_owned(),
        );
        write!(stream, "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
      }
      requests
    });
    (address, task)
  }
  #[test]
  fn manual_scan_retries_after_phone_error_and_reports_lush4() {
    let (address, task) = mock_phone(vec![
      (200, r#"{"code":400,"message":"not ready"}"#),
      (
        200,
        r#"{"code":"200","data":{"toy":{"id":"lush-id","name":"Lush 4","status":"1","battery":"87"}}}"#,
      ),
    ]);
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(async {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        let manager = LovenseConnectServiceCommunicationManager::with_hosts(sender, vec![address]);
        manager.scan().await.unwrap();
        assert!(receiver.try_recv().is_err());
        manager.scan().await.unwrap();
        match receiver.recv().await.unwrap() {
          HardwareCommunicationManagerEvent::DeviceFound { name, address, .. } => {
            assert_eq!(name, "Lush");
            assert_eq!(address, "lush-id");
          }
          _ => panic!("Expected Lush device"),
        }
      });
    assert_eq!(
      task.join().unwrap(),
      vec!["GET /GetToys HTTP/1.1", "GET /GetToys HTTP/1.1"]
    );
  }
  #[test]
  fn discovered_lush_hardware_transmits_vibration_and_stop_with_encoded_id() {
    use buttplug_server::device::hardware::{HardwareReadCmd, HardwareWriteCmd};
    use buttplug_server_device_config::Endpoint;
    let (address, task) = mock_phone(vec![
      (
        200,
        r#"{"code":200,"data":{"toy":{"id":"lush&x=1","name":"Lush","status":1,"battery":87}}}"#,
      ),
      (200, r#"{"code":200}"#),
      (200, r#"{"code":200}"#),
    ]);
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(async {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        let manager = LovenseConnectServiceCommunicationManager::with_hosts(sender, vec![address]);
        manager.scan().await.unwrap();
        let HardwareCommunicationManagerEvent::DeviceFound { mut creator, .. } =
          receiver.recv().await.unwrap()
        else {
          panic!("Expected device");
        };
        let mut specializer = creator.connect().await.unwrap();
        let hardware = specializer.specialize(&[]).await.unwrap();
        let battery = hardware
          .read_value(&HardwareReadCmd::new(
            uuid::Uuid::new_v4(),
            Endpoint::Rx,
            0,
            0,
          ))
          .await
          .unwrap();
        assert_eq!(battery.data(), &[87]);
        for speed in [10, 0] {
          let command = format!("Vibrate1?v={speed}&t=lush&x=1");
          hardware
            .write_value(&HardwareWriteCmd::new(
              &[uuid::Uuid::new_v4()],
              Endpoint::Tx,
              command.into_bytes(),
              false,
            ))
            .await
            .unwrap();
        }
      });
    assert_eq!(
      task.join().unwrap(),
      vec![
        "GET /GetToys HTTP/1.1",
        "GET /Vibrate1?v=10&t=lush%26x%3D1 HTTP/1.1",
        "GET /Vibrate1?v=0&t=lush%26x%3D1 HTTP/1.1",
      ]
    );
  }
  #[test]
  fn buttplug_game_commands_reach_lush_and_stop() {
    use buttplug_server::{
      ButtplugServerBuilder,
      device::ServerDeviceManagerBuilder,
      message::{
        ButtplugClientMessageV3, ButtplugClientMessageVariant, ButtplugServerMessageV3,
        ButtplugServerMessageVariant,
      },
    };
    use futures::{StreamExt, pin_mut};
    struct TestManagerBuilder(String);
    impl HardwareCommunicationManagerBuilder for TestManagerBuilder {
      fn finish(
        &mut self,
        sender: Sender<HardwareCommunicationManagerEvent>,
      ) -> Box<dyn HardwareCommunicationManager> {
        Box::new(TimedRetryCommunicationManager::new(
          LovenseConnectServiceCommunicationManager::with_hosts(sender, vec![self.0.clone()]),
        ))
      }
    }
    let (address, task) = mock_phone(vec![
      (
        200,
        r#"{"code":200,"data":{"toy":{"id":"lush-id","name":"Lush 4","status":1,"battery":87}}}"#,
      ),
      (200, r#"{"code":200}"#),
      (200, r#"{"code":200}"#),
    ]);
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
      let config = buttplug_server_device_config::load_protocol_configs(&None, &None, false).unwrap().finish().unwrap();
      let mut manager = ServerDeviceManagerBuilder::new(config);
      manager.comm_manager(TestManagerBuilder(address));
      let server = ButtplugServerBuilder::new(manager.finish().unwrap()).finish().unwrap();
      let events = server.event_stream(); pin_mut!(events);
      async fn send(server: &buttplug_server::ButtplugServer, text: &str) {
        let message: ButtplugClientMessageV3 = serde_json::from_str(text).unwrap();
        let result = server.parse_message(ButtplugClientMessageVariant::V3(message)).await;
        assert!(result.is_ok(), "{result:?}");
      }
      send(&server, r#"{"RequestServerInfo":{"Id":1,"ClientName":"Lush game test","MessageVersion":3}}"#).await;
      send(&server, r#"{"StartScanning":{"Id":2}}"#).await;
      let index = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
          if let Some(ButtplugServerMessageVariant::V3(ButtplugServerMessageV3::DeviceAdded(device))) = events.next().await {
            assert_eq!(device.device_name(), "Lovense Lush");
            return device.device_index();
          }
        }
      }).await.expect("Buttplug must discover Lush 4 through manual phone address");
      send(&server, &format!(r#"{{"ScalarCmd":{{"Id":3,"DeviceIndex":{index},"Scalars":[{{"Index":0,"Scalar":0.5,"ActuatorType":"Vibrate"}}]}}}}"#)).await;
      // Device task batching is asynchronous; let the actuation reach the phone before stopping.
      tokio::time::sleep(Duration::from_millis(150)).await;
      send(&server, r#"{"StopAllDevices":{"Id":4}}"#).await;
      tokio::time::sleep(Duration::from_millis(150)).await;
      send(&server, r#"{"StopScanning":{"Id":5}}"#).await;
    });
    let requests = task.join().unwrap();
    assert_eq!(requests[0], "GET /GetToys HTTP/1.1");
    assert!(
      requests.contains(&"GET /Vibrate1?v=10&t=lush-id HTTP/1.1".to_owned()),
      "{requests:?}"
    );
    assert!(
      requests.contains(&"GET /Vibrate1?v=0&t=lush-id HTTP/1.1".to_owned()),
      "{requests:?}"
    );
  }
  #[test]
  fn connected_device_reports_disconnect_when_phone_no_longer_lists_it() {
    use buttplug_server::device::hardware::{HardwareConnector, HardwareEvent};
    let (address, task) = mock_phone(vec![(200, r#"{"code":200,"data":{}}"#)]);
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(async {
        let toy = LovenseServiceToyInfo {
          id: "lush-id".into(),
          name: "Lush".into(),
          connected: true,
          battery: 87,
        };
        let mut connector = LovenseServiceHardwareConnector::new(&address, &toy);
        let mut specializer = connector.connect().await.unwrap();
        let hardware = specializer.specialize(&[]).await.unwrap();
        let mut events = hardware.event_stream();
        let event = tokio::time::timeout(Duration::from_secs(7), events.recv())
          .await
          .unwrap()
          .unwrap();
        assert!(matches!(event, HardwareEvent::Disconnected(id) if id == "lush-id"));
        assert!(
          tokio::time::timeout(Duration::from_millis(100), events.recv())
            .await
            .is_err()
        );
      });
    assert_eq!(task.join().unwrap(), vec!["GET /GetToys HTTP/1.1"]);
  }
  #[test]
  fn commands_require_phone_acknowledgement() {
    let (address, task) = mock_phone(vec![
      (200, r#"{"code":200}"#),
      (200, r#"{"code":400}"#),
      (503, r#"{"code":200}"#),
    ]);
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(async {
        let url = Url::parse(&format!("{address}/Vibrate?v=10&t=lush-id")).unwrap();
        assert!(send_local_command(url.clone()).await.is_ok());
        assert!(send_local_command(url.clone()).await.is_err());
        assert!(send_local_command(url).await.is_err());
      });
    let requests = task.join().unwrap();
    assert!(
      requests
        .iter()
        .all(|r| r == "GET /Vibrate?v=10&t=lush-id HTTP/1.1")
    );
  }
  #[test]
  fn validates_local_addresses_and_rejects_paths_credentials_and_public_hosts() {
    assert_eq!(
      validate_manual_host(" http://192.168.1.5:20010/ ").unwrap(),
      "http://192.168.1.5:20010"
    );
    for bad in [
      "https://192.168.1.5:20010",
      "http://example.com:20010",
      "http://8.8.8.8:20010",
      "http://192.168.1.5:0",
      "http://user:pass@192.168.1.5:20010",
      "http://192.168.1.5:20010/commands",
      "http://192.168.1.5:20010/?v=20",
    ] {
      assert!(validate_manual_host(bad).is_err(), "{bad}");
    }
  }
  #[test]
  fn iphone_string_fields_null_battery_and_lush4_name() {
    let text =
      r#"{"code":"200","data":{"toy":{"name":"Lush 4","status":"1","battery":null},"bad":{}}}"#;
    let info = parse_local_info(text).unwrap();
    assert_eq!(info.data.len(), 1);
    let toy = &info.data["toy"];
    assert_eq!(toy.id, "toy");
    assert_eq!(toy.name, "Lush");
    assert!(toy.connected);
    assert_eq!(toy.battery, 0);
    let info = parse_local_info(r#"{"code":200,"data":"{\"toy\":{\"id\":\"toy\",\"name\":\"Lush\",\"status\":true,\"battery\":\"87\"}}"}"#).unwrap();
    assert_eq!(info.data["toy"].battery, 87);
  }
  #[test]
  fn rejects_api_errors_and_invalid_data_without_panicking() {
    for text in [
      "not json",
      r#"{"code":400,"data":{}}"#,
      r#"{"code":200}"#,
      r#"{"code":200,"data":null}"#,
    ] {
      assert!(parse_local_info(text).is_err());
    }
    assert!(discovery_hosts("not json").is_err());
    assert!(discovery_hosts(r#"{"code":400}"#).is_err());
    assert_eq!(
      discovery_hosts(r#"{"192-168-1-5.lovense.club":{"httpPort":"20010"}}"#).unwrap(),
      vec!["http://192.168.1.5:20010"]
    );
  }
}
