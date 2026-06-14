use std::path::PathBuf;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

use wincompatlib::prelude::*;

use super::loader::ComponentsLoader;


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub title: String,
    pub managed: bool,
    pub versions: Vec<Version>
}

impl Group {
    /// Find wine group with given name in components index
    ///
    /// This method will also check all version names within this group, so both `wine-ge-proton` and `lutris-GE-Proton7-37-x86_64` will work
    pub fn find_in<T: Into<PathBuf>, F: AsRef<str>>(components: T, name: F) -> anyhow::Result<Option<Self>> {
        let name = name.as_ref();

        for group in get_groups(components)? {
            if group.name == name || group.versions.iter().any(move |version| version.name == name) {
                return Ok(Some(group));
            }
        }

        Ok(None)
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Bundle {
    Proton
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub files: Files,
}

impl Version {
    pub fn get_runner_dir<T: Into<PathBuf>>(&self, builds_dir: T) -> PathBuf {
        return builds_dir.into().join(&self.name);
    }

    /// Get latest recommended wine version
    pub fn latest<T: Into<PathBuf>>(components: T) -> anyhow::Result<Self> {
        Ok(get_groups(components)?[0].versions[0].clone())
    }

    /// Find wine version with given name in components index
    pub fn find_in<T: Into<PathBuf>, F: AsRef<str>>(components: T, name: F) -> anyhow::Result<Option<Self>> {
        let name = name.as_ref();

        for group in get_groups(components)? {
            if let Some(version) = group.versions.into_iter().find(move |version| version.name == name) {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    /// Find wine group current version belongs to
    pub fn find_group<T: Into<PathBuf>>(&self, components: T) -> anyhow::Result<Option<Group>> {
        let name = self.name.as_str();

        for group in get_groups(components)? {
            if group.versions.iter().any(move |version| version.name == name) {
                return Ok(Some(group));
            }
        }

        Ok(None)
    }

    #[inline]
    /// Check is current wine downloaded in specified folder
    pub fn is_downloaded_in<T: Into<PathBuf>>(&self, folder: T) -> bool {
        folder.into().join(&self.name).exists()
    }

    /// Return this version's features if they persist, or
    /// try to return group's features otherwise
    pub fn features<T: Into<PathBuf>>(&self, components: T) -> anyhow::Result<Option<Features>> {
        if self.features.is_some() {
            Ok(self.features.clone())
        }

        else {
            match self.find_group(components)? {
                Some(group) => Ok(group.features),
                None => Ok(None)
            }
        }
    }

    /// Convert current wine struct to one from `wincompatlib`
    ///
    /// `wine_folder` should point to the folder with wine binaries, so e.g. `/path/to/runners/wine-proton-ge-7.11`
    pub fn to_wine<T: Into<PathBuf>>(&self, components: T, wine_folder: Option<T>) -> UnifiedWine {
        let mut wine_folder = wine_folder.map(|folder| folder.into()).unwrap_or_default();
        if self.managed {
            wine_folder = PathBuf::from(&self.uri); // known case: if the proton install is managed, the URI is actually the local install.
        }

        let (wine, mut arch) = match self.files.wine64.as_ref() {
            Some(wine) => (wine, WineArch::Win64),
            None => (&self.files.wine, WineArch::Win32)
        };

        let wineboot = self.files.wineboot.as_ref().map(|wineboot| {
            let wineboot = PathBuf::from(wineboot);

            if let Some(ext) = wineboot.extension() {
                if ext == "exe" {
                    return WineBoot::Windows(wine_folder.join(wineboot));
                }
            }

            WineBoot::Unix(wine_folder.join(wineboot))
        });

        let wineserver = self.files.wineserver.as_ref().map(|wineserver| wine_folder.join(wineserver));

        if let Ok(Some(features)) = self.features(components) {
            if let Some(feature_arch) = features.arch {
                arch = feature_arch;
            }

            if let Some(Bundle::Proton) = features.bundle {
                let mut proton = Proton::new(wine_folder, features.managed_prefix.clone())
                    .with_arch(arch);

                // Small workaround. Most of stuff will work with just this
                proton.steam_client_path = Some(PathBuf::from(""));

                return UnifiedWine::Proton(proton);
            }
        }

        let mut wine = Wine::from_binary(wine_folder.join(wine))
            .with_loader(WineLoader::Current)
            .with_arch(arch);

        if let Some(wineboot) = wineboot {
            wine = wine.with_boot(wineboot);
        }

        if let Some(wineserver) = wineserver {
            wine = wine.with_server(wineserver);
        }

        UnifiedWine::Default(wine)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Files {
    pub run: String,
    pub v2_entry_point: String,
}

#[inline]
pub fn get_groups<T: Into<PathBuf>>(components: T) -> anyhow::Result<Vec<Group>> {
    ComponentsLoader::new(components).get_steamrt_versions()
}

/// List downloaded wine versions in some specific folder
pub fn get_downloaded<T: Into<PathBuf>>(components: T, folder: T) -> anyhow::Result<Vec<Group>> {
    let mut downloaded = Vec::new();

    let folder: PathBuf = folder.into();

    for mut group in get_groups(components)? {
        if group.managed {
            // special case: Runners are externally managed.
            downloaded.push(group);
            continue;
        }
        group.versions.retain(|version| folder.join(&version.name).exists());

        if !group.versions.is_empty() {
            downloaded.push(group);
        }
    }

    Ok(downloaded)
}
