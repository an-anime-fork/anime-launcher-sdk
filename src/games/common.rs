use std::path::PathBuf;
use crate::integrations::steam;

/// Get assumed Steam Prefix install path
///
/// Generate a sane, possible, "relative to the prefix's C:\ root" install target for games that
///  need such a location to install the game(s) in.
/// TODO: fix autodetect. This assumes a customised install; Houkai 3rd Steam Integration wants 
///        to check if we have a Steam-controlled game already installed.
pub fn base_install_dir(launcher_dir: PathBuf) -> anyhow::Result<PathBuf> {
    match steam::launched_from() {
        steam::LaunchedFrom::Independent => {
            Ok(launcher_dir)
        },
        steam::LaunchedFrom::Steam => {
            Ok(match steam::get_steam_compatdata_cdrive_root() {
                Some(path) => PathBuf::from(path),
                None => launcher_dir
            })
        }
    }
}