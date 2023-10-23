use crate::model::runer::Rune;
use anyhow::{anyhow, Result};
use config::{Config, File, FileFormat};
use log::info;

/// It expects a string literal that should correspond to a filename with
/// .runer extension.
///
/// * Returns error if the given file is not a .runer file.
/// * Returns error if it can't deserialize the given file into a valid
/// Rune struct.
///
/// MENTAL NOTE: .runer files are basically files written in valid yaml
/// format. That's why funtion currently uses yaml formatter of config
/// crate. It would be a good idea to implement a .runer specific validation
/// on top of that.
pub fn extract_rune(file: &str) -> Result<Rune> {
    if !file.ends_with(".runer") {
        return Err(anyhow!("Not a .runer file"));
    }
    let rune = Config::builder()
        .add_source(File::new(file, FileFormat::Yaml))
        .build()
        .unwrap()
        .try_deserialize::<Rune>()?;

    Ok(rune)
}

/// Utility function to see the details and metrics about the given Rune.
///
/// It logs its analysis at INFO level.
pub fn analyze_fragments(rune: &Rune) {
    if let Some(blueprints) = &rune.blueprints {
        info!("Rune has {} blueprints", { blueprints.len() });
    }

    if let Some(flows) = &rune.flows {
        info!("Rune has {} flows", { flows.len() });
    }

    if let Some(env) = &rune.env {
        info!("Rune has {} environment variable", { env.len() });
    }
}
