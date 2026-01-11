use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref ALIAS_MAP: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

pub fn add_alias(name: &str, value: &str) {
    let mut map = ALIAS_MAP.lock().unwrap();
    map.insert(name.to_string(), value.to_string());
}

pub fn resolve_alias(name: &str) -> String {
    let map = ALIAS_MAP.lock().unwrap();
    map.get(name).cloned().unwrap_or_else(|| name.to_string())
}

// Questa è la funzione che mancava!
pub fn get_all_aliases() -> Vec<(String, String)> {
    let map = ALIAS_MAP.lock().unwrap();
    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}
