// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::lovense_connect_service_comm_manager::{
  LovenseServiceToyInfo, get_local_info, send_local_command,
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::{
  GenericHardwareSpecializer, Hardware, HardwareConnector, HardwareEvent, HardwareInternal,
  HardwareReadCmd, HardwareReading, HardwareSpecializer, HardwareSubscribeCmd,
  HardwareUnsubscribeCmd, HardwareWriteCmd,
};
use buttplug_server_device_config::{
  Endpoint, LovenseConnectServiceSpecifier, ProtocolCommunicationSpecifier,
};
use futures::future::{self, BoxFuture, FutureExt};
use std::{
  fmt::{self, Debug},
  sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
  },
  time::Duration,
};
use tokio::sync::broadcast;

pub struct LovenseServiceHardwareConnector {
  http_host: String,
  toy_info: LovenseServiceToyInfo,
}

impl LovenseServiceHardwareConnector {
  pub(super) fn new(http_host: &str, toy_info: &LovenseServiceToyInfo) -> Self {
    debug!("Emitting a new lovense service hardware connector!");
    Self {
      http_host: http_host.to_owned(),
      toy_info: toy_info.clone(),
    }
  }
}

impl Debug for LovenseServiceHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LovenseServiceHardwareConnector").finish()
  }
}

#[async_trait]
impl HardwareConnector for LovenseServiceHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::LovenseConnectService(LovenseConnectServiceSpecifier::default())
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let hardware_internal =
      LovenseServiceHardware::new(&self.http_host, &self.toy_info.id, self.toy_info.battery);
    let hardware = Hardware::new(
      &self.toy_info.name,
      &self.toy_info.id,
      &[Endpoint::Tx],
      &None,
      false,
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

#[derive(Clone, Debug)]
pub struct LovenseServiceHardware {
  event_sender: broadcast::Sender<HardwareEvent>,
  http_host: String,
  battery_level: Arc<AtomicU8>,
}

impl LovenseServiceHardware {
  fn new(http_host: &str, toy_id: &str, initial_battery: i8) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    let sender_clone = device_event_sender.clone();
    let toy_id = toy_id.to_owned();
    let host = http_host.to_owned();
    let battery_level = Arc::new(AtomicU8::new(initial_battery.clamp(0, 100) as u8));
    let battery_level_clone = Arc::downgrade(&battery_level);
    buttplug_core::spawn!("LovenseServiceHardware loop", async move {
      loop {
        // SutekhVRC/VibeCheck patch for delay because Lovense Connect HTTP servers crash (Perma DOS)
        tokio::time::sleep(Duration::from_secs(5)).await;
        let Some(battery_level) = battery_level_clone.upgrade() else {
          break;
        };
        match get_local_info(&host).await {
          Some(info) => match info
            .data
            .values()
            .find(|toy| toy.id == toy_id && toy.connected)
          {
            Some(toy) => battery_level.store(toy.battery.clamp(0, 100) as u8, Ordering::Relaxed),
            None => {
              let _ = sender_clone.send(HardwareEvent::Disconnected(toy_id.clone()));
              break;
            }
          },
          None => {
            let _ = sender_clone.send(HardwareEvent::Disconnected(toy_id.clone()));
            break;
          }
        }
      }
    });
    Self {
      event_sender: device_event_sender,
      http_host: http_host.to_owned(),
      battery_level,
    }
  }
}

impl HardwareInternal for LovenseServiceHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }

  // Assume the only thing we'll read is battery.
  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    let battery_level = self.battery_level.clone();
    async move {
      Ok(HardwareReading::new(
        Endpoint::Rx,
        &[battery_level.load(Ordering::Relaxed)],
      ))
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let host = self.http_host.clone();
    let command = msg.data().clone();
    async move {
      let command = std::str::from_utf8(&command)
        .map_err(|err| ButtplugDeviceError::UnhandledCommand(err.to_string()))?;
      let (endpoint, query) = command.split_once('?').unwrap_or((command, ""));
      let mut url = reqwest::Url::parse(&format!("{host}/{endpoint}"))
        .map_err(|err| ButtplugDeviceError::UnhandledCommand(err.to_string()))?;
      // Protocol commands are strings; encode the toy id rather than allowing it
      // to create additional query parameters. All current commands place t last.
      if let Some((before, toy)) = query.split_once("t=") {
        for pair in before
          .trim_end_matches('&')
          .split('&')
          .filter(|p| !p.is_empty())
        {
          if let Some((key, value)) = pair.split_once('=') {
            url.query_pairs_mut().append_pair(key, value);
          }
        }
        url.query_pairs_mut().append_pair("t", toy);
      } else if !query.is_empty() {
        return Err(ButtplugDeviceError::UnhandledCommand(
          "Missing Lovense toy id".into(),
        ));
      }
      send_local_command(url).await
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Lovense Connect does not support subscribe".to_owned(),
    )))
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Lovense Connect does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}
