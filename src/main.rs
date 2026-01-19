mod shell;
mod config;
mod completion;

use shell::{builtins, exec, state};
use shell::repl::repl_loop;
use config::grshrc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use std::io::{self, Write, IsTerminal};
use std::os::unix::io::AsRawFd;

// Import necessari per la patch TTY e Segnali
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::unistd::{self, Pid};

lazy_static! {
    static ref ALIASES: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref SKIP_BLOCK: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

fn apply_cursor_style(style: u8) {
    if style == 0 { return; }
    let mut stdout = io::stdout();
    let _ = stdout.write_all(&[27]);
    let _ = write!(stdout, "[{} q", style);
    let _ = stdout.flush();
}

fn get_preferred_cursor() -> u8 {
    state::get_var("GRSH_CURSOR")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
}

fn split_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;

    for c in line.chars() {
        match c {
            '"' if !in_single_quotes => in_double_quotes = !in_double_quotes,
            '\'' if !in_double_quotes => in_single_quotes = !in_single_quotes,
            ' ' if !in_double_quotes && !in_single_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() { args.push(current); }
    args
}

fn run_line(line: String) -> bool {
    let raw_line = line.trim().to_string();
    if raw_line.is_empty() || raw_line.starts_with('#') { return true; }

    {
        let mut skip = SKIP_BLOCK.lock().unwrap();
        if raw_line.starts_with("endif") { *skip = false; return true; }
        if raw_line == "if" || raw_line.starts_with("if ") {
            *skip = !raw_line.contains("$?prompt");
            return true;
        }
        if *skip { return true; }
    }

    if ["bindkey", "set filec", "set history", "set autolist"].iter().any(|&c| raw_line.starts_with(c)) {
        return true;
    }

    let mut expanded_line = state::expand_env_vars(&raw_line);

    // --- PATCH ALIAS: Risoluzione immediata ---
    let first_word = expanded_line.split_whitespace().next().unwrap_or("").to_string();
    {
        let aliases = ALIASES.lock().unwrap();
        if let Some(alias_value) = aliases.get(&first_word) {
            expanded_line = expanded_line.replacen(&first_word, alias_value, 1);
        }
    }
    // ------------------------------------------

    // Gestione Pipes e Redirezioni
    if expanded_line.contains('|') || expanded_line.contains('>') ||
       expanded_line.contains('<') || expanded_line.contains(" && ") {
        let success = exec::execute(&expanded_line.replace("|&", "|"));
        state::set_exit_status(success);
        return success;
    }

    let parts = split_args(&expanded_line);
    if parts.is_empty() { return true; }

    let cmd = parts[0].as_str();
    let args: Vec<&str> = parts.iter().skip(1).map(|s| s.as_str()).collect();

    let success = match cmd {
        "source" => {
            if let Some(path) = args.first() { run_file(path); true }
            else { eprintln!("grsh: source: specificare un file"); false }
        },
        "echo" => {
            let mut is_n = false;
            let msg: Vec<String> = args.iter()
                .filter(|&&a| if a == "-n" { is_n = true; false } else { true })
                .map(|&a| a.trim_matches('"').trim_matches('\'').to_string())
                .collect();
            let output = msg.join(" ");
            if is_n { print!("{}", output); let _ = io::stdout().flush(); }
            else { println!("{}", output); }
            true
        },
        "set" | "setenv" => {
            if args.len() >= 2 {
                let key = args[0].trim_matches('(').trim_matches(')');
                let val = if args.len() > 2 && args[1] == "=" { args[2] } else { args[1] };
                let final_val = val.trim_matches('"').trim_matches('\'').trim_matches(')');
                state::set_var(key, final_val);
                if key == "GRSH_CURSOR" {
                    if let Ok(num) = final_val.parse::<u8>() { apply_cursor_style(num); }
                }
            }
            true
        },
        "alias" => {
            if let Some((name, val)) = expanded_line.replacen("alias ", "", 1).split_once('=') {
                ALIASES.lock().unwrap().insert(name.trim().to_string(), val.trim().to_string());
            } else if args.len() >= 2 {
                ALIASES.lock().unwrap().insert(args[0].to_string(), args[1..].join(" "));
            }
            true
        },
        "exit" | "quit" => std::process::exit(0),
        _ => {
            if builtins::handle_builtin(cmd, &args) {
                true
            } else {
                exec::execute(&expanded_line)
            }
        }
    };

    state::set_exit_status(success);
    success
}

fn run_file(path: &str) {
    let full_path = if path.starts_with('~') {
        path.replacen('~', &std::env::var("HOME").unwrap_or_default(), 1)
    } else { path.to_string() };

    if let Ok(content) = std::fs::read_to_string(&full_path) {
        for line in content.lines() { run_line(line.to_string()); }
    } else {
        eprintln!("grsh: errore nel leggere il file: {}", path);
    }
}

fn main() {
    let is_atty = io::stdin().is_terminal();

    if is_atty {
        unsafe {
            let ignore_action = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
            let _ = sigaction(Signal::SIGTTOU, &ignore_action);
            let _ = sigaction(Signal::SIGINT, &ignore_action);
            let _ = sigaction(Signal::SIGTSTP, &ignore_action);
            let _ = sigaction(Signal::SIGQUIT, &ignore_action);

            let shell_pid = unistd::getpid();
            let _ = unistd::setpgid(shell_pid, shell_pid);
            let _ = unistd::tcsetpgrp(io::stdin().as_raw_fd(), shell_pid);
        }
    }

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "-c" => { if args.len() > 2 { run_line(args[2].clone()); } return; },
            "--version" | "-v" | "version" => { builtins::handle_builtin("version", &[]); return; },
            "--help" | "-h" | "help" => { builtins::handle_builtin("help", &[]); return; },
            arg if arg.starts_with('-') => {
                eprintln!("grsh: flag sconosciuto: {}", arg);
                std::process::exit(1);
            },
            filename => { run_file(filename); return; }
        }
    }

    for line in grshrc::load() { run_line(line); }

    let term = std::env::var("TERM").unwrap_or_default();
    apply_cursor_style(get_preferred_cursor());

    if !is_atty || term == "" || term == "dumb" {
        return;
    } else {
        repl_loop(run_line);
    }
}
