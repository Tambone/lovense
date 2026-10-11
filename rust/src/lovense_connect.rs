//! Desktop-only persisted Lovense Connect settings, without changing the generated FFI ABI.
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Settings {
  #[serde(default)]
  address: String,
}

pub(crate) fn hosts(enabled: bool, user_device_config_path: Option<&str>) -> Result<Vec<String>> {
  if !enabled {
    return Ok(vec![]);
  }
  let Some(path) = user_device_config_path else {
    return Ok(vec![]);
  };
  let Some(parent) = Path::new(path).parent() else {
    return Ok(vec![]);
  };
  let settings_path = parent.join("lovense-connect.json");
  let contents = match std::fs::read_to_string(&settings_path) {
    Ok(contents) => contents,
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
    Err(err) => return Err(err).context("Reading Lovense Connect settings"),
  };
  let settings: Settings = serde_json::from_str(&contents)
    .context("Invalid lovense-connect.json; save the phone address in Settings again")?;
  if settings.address.trim().is_empty() {
    Ok(vec![])
  } else {
    Ok(vec![settings.address.trim().to_owned()])
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn disabled_or_missing_path_uses_discovery() {
    assert!(
      hosts(false, Some("/unreadable/device-config.json"))
        .unwrap()
        .is_empty()
    );
    assert!(hosts(true, None).unwrap().is_empty());
  }
  #[test]
  fn settings_read_from_user_config_directory() {
    let dir = std::env::temp_dir().join(format!("intiface-lovense-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let user_config = dir.join("user-device-config.json");
    assert!(hosts(true, user_config.to_str()).unwrap().is_empty());
    std::fs::write(
      dir.join("lovense-connect.json"),
      r#"{"address":"http://192.168.1.50:20010"}"#,
    )
    .unwrap();
    assert_eq!(
      hosts(true, user_config.to_str()).unwrap(),
      vec!["http://192.168.1.50:20010"]
    );
    std::fs::write(dir.join("lovense-connect.json"), r#"{"address":""}"#).unwrap();
    assert!(hosts(true, user_config.to_str()).unwrap().is_empty());
    std::fs::write(dir.join("lovense-connect.json"), "broken").unwrap();
    assert!(hosts(true, user_config.to_str()).is_err());
    std::fs::remove_dir_all(dir).unwrap();
  }
}
