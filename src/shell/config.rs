use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

pub struct GrshConfig {
    pub aliases: HashMap<String, String>,
}

pub fn load_grshrc() -> GrshConfig {
    let mut config = GrshConfig { aliases: HashMap::new() };
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".grshrc");

    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }

            // Supporto Alias: alias ll="ls -la"
            if line.starts_with("alias ") {
                let parts: Vec<&str> = line.trim_start_matches("alias ").splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim().trim_matches('"').trim_matches('\'');
                    config.aliases.insert(key.to_string(), value.to_string());
                }
            }
        }
    }
    config
}
