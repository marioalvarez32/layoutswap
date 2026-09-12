//! `load() -> Config` and `save(Config)` against one JSON file.
//!
//! The root is injected so tests run against a temporary directory. A missing file loads
//! as defaults; a file with a newer schema version than [`SCHEMA_VERSION`] is refused with
//! a message that names the file, and older versions are migrated forward before parsing.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Config, WindowSize, SCHEMA_VERSION};
use crate::error::AppError;

pub const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    /// A store whose file is `<root>/config.json`. The root is created on first save.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        ConfigStore {
            path: root.into().join(CONFIG_FILE_NAME),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The app root the config file sits in; every other file the app writes lives
    /// under it too.
    pub fn root(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }

    /// Defaults when the file does not exist; otherwise the file, migrated to the current
    /// schema version.
    pub fn load(&self) -> Result<Config, AppError> {
        match self.read(&self.path) {
            Err(AppError::ConfigRead { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                Ok(Config::default())
            }
            other => other,
        }
    }

    /// Reads a config from any file, through the same parsing and migrations as `load`,
    /// without touching this store's own file. Import goes through here.
    pub fn read(&self, path: &Path) -> Result<Config, AppError> {
        let text = fs::read_to_string(path).map_err(|source| AppError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
        // An editor that saves UTF-8 with a byte-order mark must not break the file.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let value: Value =
            serde_json::from_str(text).map_err(|source| AppError::ConfigInvalid {
                path: path.to_path_buf(),
                source,
            })?;
        let migrated = migrate(value, path)?;
        serde_json::from_value(migrated).map_err(|source| AppError::ConfigInvalid {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Writes the whole config. The write goes to a sibling temp file first and is renamed
    /// over the target, so a crash mid-write cannot leave a half file behind.
    pub fn save(&self, config: &Config) -> Result<(), AppError> {
        let write_error = |source| AppError::ConfigWrite {
            path: self.path.clone(),
            source,
        };
        if let Some(root) = self.path.parent() {
            fs::create_dir_all(root).map_err(write_error)?;
        }
        let text =
            serde_json::to_string_pretty(config).map_err(|source| AppError::ConfigInvalid {
                path: self.path.clone(),
                source,
            })?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, text).map_err(write_error)?;
        fs::rename(&temp, &self.path).map_err(write_error)
    }

    /// Load, replace the window size, save.
    pub fn update_window_size(&self, size: WindowSize) -> Result<(), AppError> {
        let mut config = self.load()?;
        config.window = size;
        self.save(&config)
    }
}

/// Brings a stored config up to [`SCHEMA_VERSION`]. Each future version adds one step
/// here that rewrites the previous shape; the chain runs from the file's version forward.
fn migrate(mut value: Value, path: &Path) -> Result<Value, AppError> {
    let invalid = |message: &str| AppError::ConfigInvalid {
        path: path.to_path_buf(),
        source: serde::de::Error::custom(message),
    };
    let found = value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid("schemaVersion is missing or not a whole number"))?;
    if found > u64::from(SCHEMA_VERSION) {
        return Err(AppError::UnknownSchemaVersion {
            found,
            supported: SCHEMA_VERSION,
            path: path.to_path_buf(),
        });
    }
    if found == 0 {
        return Err(invalid("schemaVersion 0 was never written by layoutswap"));
    }
    if found < 2 {
        migrate_1_to_2(&mut value);
    }
    if found < 3 {
        migrate_2_to_3(&mut value);
    }
    if let Some(object) = value.as_object_mut() {
        object.insert("schemaVersion".into(), Value::from(SCHEMA_VERSION));
    }
    Ok(value)
}

/// Version 3 added the capabilities map, empty until the first read.
fn migrate_2_to_3(value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object
            .entry("capabilities")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
    }
}

/// Version 2 gave every layout a timeline: empty steps, the default timings, the
/// stop fallback, and an updated-at stamp equal to its capture.
fn migrate_1_to_2(value: &mut Value) {
    use super::layouts::{DEFAULT_AVAILABLE_WAIT_SECONDS, DEFAULT_DROP_WAIT_SECONDS};
    let Some(layouts) = value.get_mut("layouts").and_then(Value::as_array_mut) else {
        return;
    };
    for layout in layouts.iter_mut().filter_map(Value::as_object_mut) {
        let captured_at = layout.get("capturedAt").cloned().unwrap_or(Value::Null);
        layout.entry("steps").or_insert_with(|| Value::Array(vec![]));
        layout
            .entry("dropWaitSeconds")
            .or_insert_with(|| Value::from(DEFAULT_DROP_WAIT_SECONDS));
        layout
            .entry("availableWaitSeconds")
            .or_insert_with(|| Value::from(DEFAULT_AVAILABLE_WAIT_SECONDS));
        layout
            .entry("onApplyFailure")
            .or_insert_with(|| Value::from("stop"));
        layout.entry("updatedAt").or_insert(captured_at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn store() -> (TempDir, ConfigStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(dir.path().join("layoutswap"));
        (dir, store)
    }

    #[test]
    fn loads_defaults_when_no_file_exists() {
        let (_dir, store) = store();
        assert_eq!(store.load().unwrap(), Config::default());
        assert!(!store.path().exists(), "load must not create the file");
    }

    #[test]
    fn round_trips_every_field() {
        let (_dir, store) = store();
        let mut config = Config {
            window: WindowSize {
                width: 1500,
                height: 900,
            },
            ..Config::default()
        };
        config.aliases.insert(
            r"\\?\DISPLAY#AUS34A1#5&abc#0#UID4353".into(),
            "Ultrawide".into(),
        );
        config.capabilities.insert(
            r"\\?\DISPLAY#AUS34A1#5&abc#0#UID4353".into(),
            crate::config::capabilities::Capabilities {
                read_at: "2026-09-11T11:52:00-05:00".into(),
                answered: true,
                input_codes: vec![0x11, 0x12, 0x0F],
                power_modes: vec![1, 5],
                modes: vec![crate::config::capabilities::Mode { width: 3440, height: 1440, hz: 100 }],
                raw: "(prot(monitor)vcp(60(11 12 0F) D6(01 05)))".into(),
            },
        );
        store.save(&config).unwrap();
        assert_eq!(store.load().unwrap(), config);
    }

    #[test]
    fn save_creates_the_root_folder_and_leaves_no_temp_file() {
        let (_dir, store) = store();
        store.save(&Config::default()).unwrap();
        assert!(store.path().exists());
        assert!(!store.path().with_extension("json.tmp").exists());
    }

    #[test]
    fn a_version_1_config_gains_a_timeline_per_layout() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        let five = include_str!("../hardware/fixtures/five-monitors.json");
        let inventory = crate::hardware::parse(five).unwrap();
        let v1 = serde_json::json!({
            "schemaVersion": 1,
            "window": { "width": 1280, "height": 860 },
            "aliases": {},
            "layouts": [{
                "id": "abc", "name": "Desk", "folder": "desk",
                "capturedAt": "2026-09-09T14:33:00-05:00",
                "arrangement": inventory.arrangement,
                "summary": crate::config::layouts::summarise(&inventory),
                "script": null
            }]
        });
        fs::write(store.path(), v1.to_string()).unwrap();
        let config = store.load().unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        let layout = &config.layouts[0];
        assert!(layout.steps.is_empty());
        assert_eq!(layout.drop_wait_seconds, 5);
        assert_eq!(layout.available_wait_seconds, 120);
        assert_eq!(layout.on_apply_failure, crate::config::layouts::ApplyFailure::Stop);
        assert_eq!(layout.updated_at, layout.captured_at);
    }

    #[test]
    fn a_version_2_config_gains_an_empty_capabilities_map() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        let v2 = serde_json::json!({
            "schemaVersion": 2,
            "window": { "width": 1280, "height": 860 },
            "aliases": { "path-a": "Side" },
            "layouts": []
        });
        fs::write(store.path(), v2.to_string()).unwrap();
        let config = store.load().unwrap();
        assert_eq!(config.schema_version, 3);
        assert!(config.capabilities.is_empty());
        assert_eq!(config.aliases.get("path-a").map(String::as_str), Some("Side"));
        let text = serde_json::to_string(&config).unwrap();
        assert!(text.contains(r#""capabilities":{}"#), "{text}");
    }

    #[test]
    fn rejects_an_unknown_schema_version_with_a_clear_error() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(
            store.path(),
            r#"{"schemaVersion": 99, "window": {"width": 1, "height": 1}}"#,
        )
        .unwrap();
        let error = store.load().unwrap_err();
        assert!(matches!(
            error,
            AppError::UnknownSchemaVersion {
                found: 99,
                supported: SCHEMA_VERSION,
                ..
            }
        ));
        let message = error.to_string();
        assert!(message.contains("schema version 99"), "{message}");
        assert!(message.contains("config.json"), "{message}");
        assert!(message.starts_with("Update layoutswap"), "{message}");
    }

    #[test]
    fn reads_a_file_saved_with_a_utf8_byte_order_mark() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(
            store.path(),
            "\u{feff}{\"schemaVersion\": 1, \"window\": {\"width\": 1000, \"height\": 700}}",
        )
        .unwrap();
        let config = store.load().unwrap();
        assert_eq!(
            config.window,
            WindowSize {
                width: 1000,
                height: 700
            }
        );
    }

    #[test]
    fn rejects_a_file_without_a_schema_version() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), r#"{"window": {"width": 1, "height": 1}}"#).unwrap();
        assert!(matches!(
            store.load().unwrap_err(),
            AppError::ConfigInvalid { .. }
        ));
    }

    #[test]
    fn rejects_malformed_json() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "{ not json").unwrap();
        assert!(matches!(
            store.load().unwrap_err(),
            AppError::ConfigInvalid { .. }
        ));
    }

    #[test]
    fn update_window_size_keeps_the_other_fields() {
        let (_dir, store) = store();
        let config = Config {
            aliases: BTreeMap::from([("path".to_string(), "Side".to_string())]),
            ..Config::default()
        };
        store.save(&config).unwrap();

        let size = WindowSize {
            width: 1024,
            height: 700,
        };
        store.update_window_size(size).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.window, size);
        assert_eq!(loaded.aliases, config.aliases);
        assert_eq!(loaded.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn the_file_is_camel_case_json_with_a_schema_version() {
        let (_dir, store) = store();
        store.save(&Config::default()).unwrap();
        let text = fs::read_to_string(store.path()).unwrap();
        assert!(text.contains(&format!(r#""schemaVersion": {SCHEMA_VERSION}"#)), "{text}");
        assert!(text.contains(r#""window""#), "{text}");
        assert!(text.contains(r#""aliases""#), "{text}");
    }
}
