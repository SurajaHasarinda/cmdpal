mod model;

pub use model::{Config, Context, Ranking, GOOGLE_BASE_URL};

use anyhow::{Context as _, Result};
use std::{env, fs, path::PathBuf};

pub const ENV_PATH: &str = "CMDPAL_CONFIG";

pub fn path() -> Result<PathBuf> {
    env::var_os(ENV_PATH)
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|dir| dir.join("cmdpal").join("config.toml")))
        .context("no config directory available")
}

pub fn load() -> Result<Config> {
    let mut config = match path() {
        Ok(file) if file.is_file() => read(&file)?,
        _ => Config::default(),
    };
    apply_env(&mut config);
    config.clamp();
    Ok(config)
}

pub fn read(file: &PathBuf) -> Result<Config> {
    let text = fs::read_to_string(file)?;
    match toml::from_str(&text) {
        Ok(config) => Ok(config),
        Err(error) => {
            eprintln!("config: {error} — using defaults");
            Ok(Config::default())
        }
    }
}

pub fn save(config: &Config) -> Result<PathBuf> {
    let file = path()?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut clean = config.clone();
    clean.unknown.clear();
    fs::write(&file, toml::to_string_pretty(&clean)?)?;
    Ok(file)
}

pub fn warnings(config: &Config) -> Vec<String> {
    config
        .unknown
        .keys()
        .map(|key| format!("unknown config key: {key}"))
        .collect()
}

fn apply_env(config: &mut Config) {
    if let Ok(model) = env::var("CMDPAL_AI_MODEL") {
        config.ai.model = model;
    }
    if let Ok(base_url) = env::var("CMDPAL_AI_BASE_URL") {
        config.ai.base_url = base_url;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, text).unwrap();
        (dir, file)
    }

    #[test]
    fn missing_file_yields_defaults() {
        let config = Config::default();
        assert!(config.ai.model.is_empty());
        assert_eq!(config.ai.base_url, GOOGLE_BASE_URL);
        assert_eq!(config.ranking.match_weight, 3.0);
    }

    #[test]
    fn partial_file_keeps_other_defaults() {
        let (_dir, file) = write("[ai]\nmodel = \"gemini-3-flash\"\n");
        let config = read(&file).unwrap();
        assert_eq!(config.ai.model, "gemini-3-flash");
        assert_eq!(config.context.max_depth, 3);
    }

    #[test]
    fn unknown_keys_are_collected_not_fatal() {
        let (_dir, file) = write("[general]\nlanguage = \"en\"\n");
        let config = read(&file).unwrap();
        assert!(warnings(&config).contains(&"unknown config key: general".to_string()));
    }

    #[test]
    fn malformed_file_falls_back_to_defaults() {
        let (_dir, file) = write("this is not = valid = toml [[[");
        let config = read(&file).unwrap();
        assert_eq!(config.context.max_files, 200);
    }

    #[test]
    fn clamp_bounds_every_numeric() {
        let mut config = Config::default();
        config.ai.timeout_secs = 0;
        config.ai.temperature = 9.0;
        config.context.max_depth = 500;
        config.ranking.demote_other_modes = 4.0;
        config.clamp();
        assert_eq!(config.ai.timeout_secs, 1);
        assert_eq!(config.ai.temperature, 2.0);
        assert_eq!(config.context.max_depth, 8);
        assert_eq!(config.ranking.demote_other_modes, 1.0);
    }

    #[test]
    fn base_url_always_ends_with_slash() {
        let mut config = Config::default();
        config.ai.base_url = "https://example.com/v1".into();
        config.clamp();
        assert_eq!(config.ai.base_url, "https://example.com/v1/");
    }

    #[test]
    fn save_round_trips_without_unknown_keys() {
        let mut config = Config::default();
        config.ai.model = "m".into();
        config
            .unknown
            .insert("junk".into(), toml::Value::Boolean(true));
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("c.toml");
        let text = toml::to_string_pretty(&clean(&config)).unwrap();
        std::fs::write(&file, &text).unwrap();
        let loaded = read(&file).unwrap();
        assert_eq!(loaded.ai.model, "m");
        assert!(loaded.unknown.is_empty());
    }

    fn clean(config: &Config) -> Config {
        let mut c = config.clone();
        c.unknown.clear();
        c
    }
}
