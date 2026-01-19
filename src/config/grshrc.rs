use std::fs;
use std::path::PathBuf;
use std::env;
use std::process::Command;
use reedline::{Prompt, PromptHistorySearch, PromptEditMode};
use nu_ansi_term::Color;
use std::borrow::Cow;
use crate::shell::state; // Importiamo lo stato per contare i job

/// Carica le righe valide dal file .grshrc nella home dell'utente.
pub fn load() -> Vec<String> {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".grshrc");

    if let Ok(content) = fs::read_to_string(path) {
        content.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    } else {
        Vec::new()
    }
}

/// Funzione per ottenere il branch Git e lo stato (dirty/clean)
fn get_git_info() -> Option<String> {
    if !std::path::Path::new(".git").exists() {
        return None;
    }

    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|out| if out.status.success() { 
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string()) 
        } else { None });

    let is_dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(false);

    branch.map(|b| {
        let status_char = if is_dirty { " ✗" } else { "" };
        format!("[{}{}]", b, status_char)
    })
}

pub struct GrshPrompt;

impl Prompt for GrshPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        let user = env::var("USER").unwrap_or_else(|_| "user".into());
        
        // --- PATCH HOSTNAME ---
        let host = Command::new("hostname")
            .arg("-s")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "freebsd".into());
        // ----------------------
        
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut pwd = current_dir.display().to_string();

        if let Some(home) = dirs::home_dir() {
            let home_str = home.display().to_string();
            if pwd.starts_with(&home_str) {
                pwd = pwd.replacen(&home_str, "~", 1);
            }
        }

        let user_color = if user == "root" { Color::Red.bold() } else { Color::Green.bold() };
        let at_host_color = Color::Green;
        let pwd_color = Color::Blue.bold();
        let git_color = Color::Fixed(208).bold(); // Arancione
        let job_color = Color::Yellow.bold();

        // Info sui JOB sospesi
        let jobs_count = state::get_jobs().len();
        let job_info = if jobs_count > 0 {
            format!("{} ", job_color.paint(format!("[{}]", jobs_count)))
        } else {
            String::new()
        };

        // Recuperiamo le info Git
        let git_info = get_git_info()
            .map(|info| format!("{} ", git_color.paint(info)))
            .unwrap_or_default();

        let prompt = format!(
            "{}{}{}{} {}{} ➜ ",
            job_info,
            user_color.paint(user),
            at_host_color.paint("@"),
            at_host_color.paint(host),
            git_info,
            pwd_color.paint(pwd),
        );

        Cow::Owned(prompt)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> { Cow::Borrowed("") }
    fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> { Cow::Borrowed("") }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> { Cow::Borrowed("::: ") }

    fn render_prompt_history_search_indicator(&self, history_search: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Owned(format!("search: {} ", history_search.term))
    }
}
