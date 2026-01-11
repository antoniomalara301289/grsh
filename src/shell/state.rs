use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use std::env;
use std::path::PathBuf;

// --- STRUTTURA PER LA JOB TABLE ---
#[derive(Clone, Debug)]
pub struct Job {
    pub id: usize,
    pub pid: i32,
    pub command: String,
    pub status: String,
}

lazy_static! {
    // Memoria per le variabili d'ambiente della sessione
    pub static ref ENV_VARS: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    
    // La Job Table per gestire i processi sospesi (Ctrl+Z)
    static ref JOBS: Mutex<Vec<Job>> = Mutex::new(Vec::new());
}

// --- GESTIONE JOB (STILE BASH) ---

/// Aggiunge un processo sospeso alla tabella con un ID incrementale corretto
pub fn add_job(pid: i32, command: String) {
    let mut jobs = JOBS.lock().unwrap();
    
    // Trova l'ID più alto attuale e aggiunge 1, o parte da 1 se la lista è vuota
    let next_id = jobs.iter().map(|j| j.id).max().unwrap_or(0) + 1;
    
    jobs.push(Job {
        id: next_id,
        pid,
        command: command.clone(),
        status: "Stopped".to_string(),
    });
    println!("\n[{}] + {} suspended", next_id, command);
}

/// Restituisce una copia di tutti i job attivi
pub fn get_jobs() -> Vec<Job> {
    JOBS.lock().unwrap().clone()
}

/// Estrae l'ultimo job inserito (usato da 'fg' senza argomenti)
pub fn pop_last_job() -> Option<Job> {
    JOBS.lock().unwrap().pop()
}

/// Estrae un job specifico tramite il suo ID (usato da 'fg <id>')
pub fn pop_job_by_id(id: usize) -> Option<Job> {
    let mut jobs = JOBS.lock().unwrap();
    if let Some(index) = jobs.iter().position(|j| j.id == id) {
        Some(jobs.remove(index))
    } else {
        None
    }
}

/// Rimuove un job specifico tramite PID (es: dopo un kill o zap)
pub fn remove_job_by_pid(pid: i32) {
    let mut jobs = JOBS.lock().unwrap();
    jobs.retain(|j| j.pid != pid);
}

/// Svuota completamente la Job Table
pub fn clear_jobs() {
    let mut jobs = JOBS.lock().unwrap();
    jobs.clear();
}

// --- GESTIONE VARIABILI ED ESPANSIONI ---

pub fn get_var(key: &str) -> Option<String> {
    let vars = ENV_VARS.lock().unwrap();
    vars.get(key).cloned()
}

pub fn set_var(key: &str, value: &str) {
    let mut vars = ENV_VARS.lock().unwrap();
    vars.insert(key.to_string(), value.to_string());
    env::set_var(key, value);
}

pub fn set_exit_status(success: bool) {
    let status = if success { "0" } else { "1" };
    set_var("?", status);
}

pub fn expand_env_vars(line: &str) -> String {
    let mut expanded = line.to_string();

    {
        let vars = ENV_VARS.lock().unwrap();
        for (key, value) in vars.iter() {
            let target = format!("${}", key);
            expanded = expanded.replace(&target, value);
        }
    }

    for (key, value) in env::vars() {
        let target = format!("${}", key);
        if expanded.contains(&target) {
            expanded = expanded.replace(&target, &value);
        }
    }

    let home = env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    
    let words: Vec<String> = expanded.split(' ').map(|word| {
        if word == "~" {
            home.clone()
        } else if word.starts_with("~/") {
            word.replacen('~', &home, 1)
        } else {
            word.to_string()
        }
    }).collect();

    words.join(" ")
}

pub fn find_in_path(cmd: &str) -> Option<PathBuf> {
    if let Ok(path_var) = env::var("PATH") {
        for path in env::split_paths(&path_var) {
            let full_path = path.join(cmd);
            if full_path.is_file() {
                return Some(full_path);
            }
        }
    }
    None
}
