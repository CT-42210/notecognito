use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::error::Result;
use crate::notecard::{Notecard, NotecardId};
use crate::platform::HotkeyModifier;

/// Display properties for notecards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayProperties {
    /// Opacity level (0-100)
    pub opacity: u8,
    /// Position on screen (x, y coordinates)
    pub position: (i32, i32),
    /// Size (width, height)
    pub size: (u32, u32),
    /// Auto-hide duration in seconds (0 for manual dismiss)
    pub auto_hide_duration: u32,
    /// Font family name
    pub font_family: String,
    /// Font size in points
    pub font_size: u32,
    /// Enable algorithmic spacing
    pub algorithmic_spacing: bool,
}

impl Default for DisplayProperties {
    fn default() -> Self {
        DisplayProperties {
            opacity: 95,
            position: (100, 100),
            size: (400, 200),
            auto_hide_duration: 0,
            font_family: "System".to_string(),
            font_size: 16,
            algorithmic_spacing: false,
        }
    }
}

/// Global application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Launch on system login
    pub launch_on_startup: bool,
    /// Default notecard settings
    pub default_display_properties: DisplayProperties,
    /// Hotkey modifier keys
    pub hotkey_modifiers: Vec<HotkeyModifier>,
    /// All notecards (keyed by ID)
    #[serde(serialize_with = "serialize_notecards", deserialize_with = "deserialize_notecards")]
    pub notecards: HashMap<NotecardId, Notecard>,
}

// Custom serialization for notecards to handle NotecardId as string keys in JSON
fn serialize_notecards<S>(
    notecards: &HashMap<NotecardId, Notecard>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(notecards.len()))?;
    for (k, v) in notecards {
        map.serialize_entry(&k.value().to_string(), v)?;
    }
    map.end()
}

fn deserialize_notecards<'de, D>(
    deserializer: D,
) -> std::result::Result<HashMap<NotecardId, Notecard>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string_map: HashMap<String, Notecard> = HashMap::deserialize(deserializer)?;
    let mut result = HashMap::new();

    for (k, v) in string_map {
        let id = k.parse::<u8>()
            .map_err(serde::de::Error::custom)?;
        let notecard_id = NotecardId::new(id)
            .map_err(serde::de::Error::custom)?;
        result.insert(notecard_id, v);
    }

    Ok(result)
}

impl Default for Config {
    fn default() -> Self {
        let mut notecards = HashMap::new();

        // Initialize with empty notecards for IDs 1-9
        for i in 1..=9 {
            let id = NotecardId::new(i).unwrap();
            notecards.insert(id, Notecard::empty(id));
        }

        Config {
            launch_on_startup: false,
            default_display_properties: DisplayProperties::default(),
            hotkey_modifiers: vec![HotkeyModifier::Control, HotkeyModifier::Shift],
            notecards,
        }
    }
}

/// Manages configuration file operations
pub struct ConfigManager {
    config_path: PathBuf,
    config: Config,
}

impl ConfigManager {
    /// Creates a new ConfigManager with the default config path
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| crate::error::NotecognitoError::Config(
                "Could not determine config directory".to_string()
            ))?;

        let app_config_dir = config_dir.join("notecognito");
        std::fs::create_dir_all(&app_config_dir)?;

        let config_path = app_config_dir.join("config.json");

        let config = if config_path.exists() {
            Self::load_from_file(&config_path)?
        } else {
            Config::default()
        };

        Ok(ConfigManager {
            config_path,
            config,
        })
    }

    /// Creates a ConfigManager with a custom config path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_path = path.as_ref().to_path_buf();

        let config = if config_path.exists() {
            Self::load_from_file(&config_path)?
        } else {
            Config::default()
        };

        Ok(ConfigManager {
            config_path,
            config,
        })
    }

    /// Loads configuration from a file
    fn load_from_file(path: &Path) -> Result<Config> {
        let contents = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Saves the current configuration to file
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(&self.config_path, json)?;
        Ok(())
    }

    /// Gets a reference to the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Gets a mutable reference to the current configuration
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Updates a notecard
    pub fn update_notecard(&mut self, notecard: Notecard) -> Result<()> {
        notecard.validate()?;
        self.config.notecards.insert(notecard.id, notecard);
        Ok(())
    }

    /// Gets a notecard by ID
    pub fn get_notecard(&self, id: NotecardId) -> Option<&Notecard> {
        self.config.notecards.get(&id)
    }
}