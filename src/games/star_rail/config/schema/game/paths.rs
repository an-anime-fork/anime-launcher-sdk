use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

use anime_game_core::star_rail::consts::GameEdition;

use crate::star_rail::consts::launcher_dir;

use crate::integrations::steam;
use crate::star_rail::consts::base_game_install_dir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paths {
    pub global: PathBuf,
    pub china: PathBuf
}

impl Paths {
    #[inline]
    /// Get game path for given edition
    pub fn for_edition(&self, edition: impl Into<GameEdition>) -> &Path {
        match edition.into() {
            GameEdition::Global => self.global.as_path(),
            GameEdition::China  => self.china.as_path()
        }
    }
}

impl Default for Paths {
    fn default() -> Self {
        let launcher_dir = base_game_install_dir().expect("Failed to get launcher dir");

        Self {
            global: match steam::launched_from() {
                steam::LaunchedFrom::Independent => launcher_dir.join("HSR"),
                steam::LaunchedFrom::Steam => launcher_dir.join("StarRail").join("Games")
            },
            china: launcher_dir.join("HSR China")
        }
    }
}

impl From<&JsonValue> for Paths {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();

        Self {
            global: match value.get("global") {
                Some(value) => match value.as_str() {
                    Some(value) => PathBuf::from(value),
                    None => default.global
                },
                None => default.global
            },

            china: match value.get("china") {
                Some(value) => match value.as_str() {
                    Some(value) => PathBuf::from(value),
                    None => default.china
                },
                None => default.china
            }
        }
    }
}
