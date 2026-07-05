use std::path::PathBuf;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

use super::loader::ComponentsLoader;


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub title: String,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Files {
    pub run: String,
    pub manifest: String,
}

#[inline]
pub fn get_groups<T: Into<PathBuf>>(components: T) -> anyhow::Result<Vec<Group>> {
    ComponentsLoader::new(components).get_steamrt_versions()
}

/// List downloaded wine versions in some specific folder
pub fn get_local_runtimes<T: Into<PathBuf>>(components: T, folder: T) -> anyhow::Result<Vec<Group>> {
    let mut localrts = Vec::new();

    let folder: PathBuf = folder.into();

    for mut group in get_groups(components)? {
        group.versions.retain(|version| folder.join(&version.name).exists());

        if !group.versions.is_empty() {
            localrts.push(group);
        }
    }

    Ok(localrts)
}
