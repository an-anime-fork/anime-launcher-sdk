use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

use anime_game_core::genshin::consts::GameEdition;

use crate::genshin::consts::launcher_dir;
use crate::integrations::steam;

use crate::genshin::consts::base_game_install_dir;

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
            GameEdition::China => self.china.as_path()
        }
    }
}

/// NOTE: These paths can be whatever we set or need.
///       These are only sane defaults provided out of
///       necessity
fn concat_gen_shin() -> String {
    format!("{}{}{} {}{}", "Ge", "nshi", "n", "Imp", "act")
}

fn concat_gen_shin_game() -> String {
    format!("{} {}", concat_gen_shin(), "game")
}

fn get_global_launchdir(launcher_dir: PathBuf) -> PathBuf {
    match steam::launched_from() {
        steam::LaunchedFrom::Independent => launcher_dir
            .join(concat_gen_shin()),
        steam::LaunchedFrom::Steam => launcher_dir
            .join(concat_gen_shin())
            .join(concat_gen_shin_game())
    }
}

impl Default for Paths {
    fn default() -> Self {
        let launcher_dir = base_game_install_dir().expect("Failed to get launcher dir");

        Self {
            global: get_global_launchdir(launcher_dir.clone()),
            china: launcher_dir.clone().join(concat!("Yu", "anS", "hen")) // TODO: autogen Steam?
        }
    }
}

impl From<&JsonValue> for Paths {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();
        match steam::aagl_launcher_launch_target() {
            None => Self {
                global: value.get("global")
                    .and_then(JsonValue::as_str)
                    .map(PathBuf::from)
                    .unwrap_or(default.global),

                china: value.get("china")
                    .and_then(JsonValue::as_str)
                    .map(PathBuf::from)
                    .unwrap_or(default.china),
            },
            Some(target_exe) => {
                let ret_path = PathBuf::from(target_exe.clone()).parent().unwrap().to_path_buf();
                Self {
                    global: ret_path.clone(),
                    china: ret_path.clone(),
                }
            }
        }

    }
}
