use std::fs;
use std::path::Path;

use crate::model::HarpeConfig;

/// Extension des fichiers de configuration de la harpe.
pub const EXTENSION: &str = "harpcfg";

/// Renvoie le chemin avec l'extension `.harpcfg`, ajoutée si absente.
pub fn avec_extension(chemin: &Path) -> std::path::PathBuf {
    match chemin.extension().and_then(|e| e.to_str()) {
        Some(e) if e.eq_ignore_ascii_case(EXTENSION) => chemin.to_path_buf(),
        _ => chemin.with_extension(EXTENSION),
    }
}

/// Sauvegarde la configuration dans un fichier JSON (extension `.harpcfg`).
pub fn sauvegarder(chemin: &Path, config: &HarpeConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(chemin, json).map_err(|e| e.to_string())
}

/// Charge une configuration depuis un fichier `.harpcfg` (ou `.json`).
pub fn charger(chemin: &Path) -> Result<HarpeConfig, String> {
    let contenu = fs::read_to_string(chemin).map_err(|e| e.to_string())?;
    let config: HarpeConfig = serde_json::from_str(&contenu).map_err(|e| e.to_string())?;
    if config.cordes.is_empty() {
        return Err("La configuration ne contient aucune corde.".to_string());
    }
    Ok(config)
}
