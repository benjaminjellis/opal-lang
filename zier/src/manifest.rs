use std::{collections::HashMap, path::PathBuf};

use eyre::Context;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::MANIFEST_NAME;

#[derive(Serialize, Deserialize)]
pub(crate) struct ZierManifest {
    pub(crate) package: Package,
    pub(crate) dependencies: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) version: Version,
}

impl ZierManifest {
    fn new(name: String) -> Self {
        Self {
            package: Package {
                name,
                version: Version::new(0, 1, 0),
            },
            dependencies: Default::default(),
        }
    }
}

pub(crate) fn read_manifest(root: PathBuf) -> eyre::Result<ZierManifest> {
    let manifest_file_path = root.join(MANIFEST_NAME);
    let file = std::fs::read(&manifest_file_path).context(format!(
        "failed to read {MANIFEST_NAME} at {manifest_file_path:?}"
    ))?;
    let manifest: ZierManifest =
        toml::from_slice(&file).context("failed to parse {MANIFEST_NAME}")?;
    Ok(manifest)
}

pub(crate) fn create_new_manifest(name: String) -> ZierManifest {
    ZierManifest::new(name)
}

pub(crate) fn write_manifest(manifest: &ZierManifest, path: &PathBuf) -> eyre::Result<()> {
    let manifest_as_string =
        toml::to_string_pretty(&manifest).context("failed to write {MANIFEST_NAME} to string")?;

    std::fs::write(path, manifest_as_string).context("failed to write {MANIFEST_NAME} to disk")?;
    Ok(())
}
