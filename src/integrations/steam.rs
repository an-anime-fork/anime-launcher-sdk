use steamlocate::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use crate::components;

#[derive(Debug, Clone, PartialEq)]
pub enum LaunchedFrom {
    Steam,
    // TODO: Heroic?
    Independent
}

#[derive(Debug, Clone, PartialEq)]
pub enum Steam {
    Desktop,
    Deck,
    OS,
    /// ...
    Invalid
}

pub fn environment() -> Steam {
    match launched_from_steam() {
        true => match is_steam_os() {
            true => match is_steam_deck() {
                true => Steam::Deck,
                false => Steam::OS
            },
            false => Steam::Desktop
        },
        false => Steam::Invalid
    }
}

pub fn launched_from() -> LaunchedFrom {
    if environment() == Steam::Invalid {
        return LaunchedFrom::Independent;
    }
    LaunchedFrom::Steam
}

fn check_env_var_for_val(env_var_key: &str, expected_value: &str) -> bool {
    match env::var_os(env_var_key) {
        Some(val) => val == expected_value,
        None => false
    }
}

/// Identify whether we were launched through a Steam environment.
fn launched_from_steam() -> bool {
    check_env_var_for_val("SteamEnv", "1")
}

/// Identify whether we are running on Steam Deck.
fn is_steam_deck() -> bool {
    check_env_var_for_val("SteamDeck", "1")
}

/// Identify whether we were launched through a SteamOS environment.
fn is_steam_os() -> bool {
    check_env_var_for_val("SteamOS", "1")
}

/// Prefix updates are disabled on Steam, as we assume the runners are Proton-spec and manage that.
pub fn is_prefix_update_disabled() -> bool {
    launched_from() == LaunchedFrom::Steam
}

pub fn default_window_size_width(default: i32) -> i32 {
    match is_steam_deck() {
        true => 1280,
        false => default
    }
}

pub fn default_window_size_height(default: i32) -> i32 {
    match is_steam_deck() {
        true => 800,
        false => default
    }
}

pub fn get_steam_compatdata_cdrive_root() -> Option<String> {
    match get_steam_compat_path() {
        None => None,
        Some(compat_path) => {
            Some(format!("{compat_path}/pfx/drive_c"))
        }
    }
}

/// Generate a list of Steam-inventoried search roots.
fn get_steam_search_roots() -> Option<Vec<PathBuf>> {
    // initialize and let Steam seed itself.
    match SteamDir::locate() {
        Some(mut steam_install_dir) => {
            Some(steam_install_dir.libraryfolders().paths
                .clone()
                .into_iter()
                .map(|single_path| single_path.join("common"))
                .chain(
                    [steam_install_dir.path.clone().join("compatibilitytools.d")]
                        .to_vec()
                        .into_iter()
                )
                .collect::<Vec<PathBuf>>()
            )
        }
        None => None
    }
}

fn check_pld(_ld: PathBuf) -> Option<PathBuf> {
    let pld = PathBuf::from(_ld);
    match pld.is_dir() // is it a directory that contains things
            && !pld.is_symlink() // is it NOT a symlink (don't inventory doppelgangers)
            && pld.join("proton").exists() // does the directory contain a proton launch script/file?
    {
        true => Some(pld),
        false => None
    }
}

fn check_root(local: PathBuf) -> Option<Vec<PathBuf>> {
    let mut processed: Vec<PathBuf> = Vec::new();
    if local.exists() && local.is_dir() {
        for _ld in local.read_dir().unwrap() {
            match check_pld(_ld.unwrap().path()) {
                Some(_pld) => processed.push(_pld),
                None => {}
            }
        }
    }
    Some(processed)
}

/// Inventory all possible Proton launchers in search roots.
fn filter_local_roots_by_proton_launcher() -> Option<Vec<PathBuf>> {
    let mut _processed: Vec<PathBuf> = Vec::new();
    match get_steam_search_roots() {
        None => { },
        Some(_locals) => {
            for _local in _locals {
                match check_root(_local) {
                    Some(_root) => _processed.extend(_root),
                    None => {}
                }
            }
        }
    }
    Some(_processed)
}

fn get_split_names(path: PathBuf) -> (Option<String>, Option<String>) {
    match fs::read_to_string(path.join("version")).expect(
        format!("Should have been able to read the file for {0}", path.display()).as_str()
    ).split_once(" ") {
        Some((_sz, proton_name)) => {
            match proton_name.is_empty() {
                false => {
                    match path.file_name() {
                        Some(file_path) => match file_path.to_str() {
                            Some(path_name) => {
                                tracing::debug!(
                                    "Identified {:?} {:?}",
                                    path_name.trim().to_string(),
                                    proton_name.trim().to_string()
                                );
                                (
                                    Some(path_name.trim().to_string()),
                                    Some(proton_name.trim().to_string())
                                )
                            },
                            None => (None, None)
                        },
                        None => (None, None)
                    }
                },
                true => (None, None)
            }
        },
        None => (None, None)
    }
}

fn get_steam_compat_path() -> Option<String> {
    match env::var("STEAM_COMPAT_DATA_PATH") {
        Ok(env_var) => Some(env_var),
        Err(_) => None
    }
}

/// Generate a list of WinCompatLib Structs for inventoried Steam-managed, detected Proton installs
pub fn get_proton_installs_as_wines() -> anyhow::Result<Vec<components::wine::Group>> {
    match filter_local_roots_by_proton_launcher() {
        Some(paths) => {
            let proton_features = components::wine::Features {
                bundle: Some(components::wine::Bundle::Proton),
                compact_launch: true,
                command: Some(String::from("python3 '%build%/proton' waitforexitandrun")),
                managed_prefix: match get_steam_compat_path() {
                    Some(val) => Some(PathBuf::from(val)),
                    None => None
                },
                ..components::wine::Features::default()
            };
            let mut wines: Vec<components::wine::Version> = Vec::new();
            for path in paths {
                let (_wine_title, _wine_name) = get_split_names(path.clone());
                match _wine_name {
                    Some(wine_name)=> match _wine_title {
                        Some(wine_title)=> {
                            // Let's gooooo!
                            wines.push(components::wine::Version {
                                name: wine_name,   // clarify
                                title: wine_title,  // clarify
                                uri: (&path.to_str().unwrap()).trim().to_string(), // clarify?
                                format: None,
                                files: components::wine::Files { // handled by wincompatlib
                                    wine: "proton".to_string(),
                                    wine64: None,
                                    wineserver: None,
                                    wineboot: None
                                },
                                // We have it already, and need it in ok later.
                                features: Some(proton_features.clone()),
                                managed: true
                            });
                        },
                        None => continue
                    },
                    None => continue
                }

            }
            Ok([
                components::wine::Group {
                    name:"steam-proton".to_string(),
                    title:"Proton Runners via Steam".to_string(),
                    features: Some(proton_features.clone()), // handled
                    versions: wines,
                    managed: true
                }
            ].to_vec())
        },
        None => Err(anyhow::anyhow!("Steam mode active but no roots?"))
    }
}

/// Get a list of Proton paths to sleuth into.
pub fn steam_proton_installed_paths() -> Option<Vec<PathBuf>> {
    match !launched_from_steam() || SteamDir::locate().is_none() {
        true => None,
        false => filter_local_roots_by_proton_launcher()
    }
}