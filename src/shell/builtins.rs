use std::env;
use std::time::Instant;
use std::os::unix::process::CommandExt;
use nu_ansi_term::Color;
use crate::shell::{alias, state};

// Import necessari per la gestione processi in fg
use nix::unistd::{self, Pid};
use nix::sys::signal::{self, Signal};
use nix::sys::wait::{waitpid, WaitStatus, WaitPidFlag};

lazy_static::lazy_static! {
    static ref SHELL_START_TIME: Instant = Instant::now();
}

pub fn handle_builtin(cmd: &str, args: &[&str]) -> bool {
    match cmd {
        // --- GESTIONE PROCESSI (JOB CONTROL) ---
        "jobs" => {
            let jobs = state::get_jobs();
            if jobs.is_empty() {
                println!("grsh: nessuna job attivo.");
            } else {
                for j in jobs {
                    println!("[{}]  {}  {}", j.id, j.status, j.command);
                }
            }
            true
        }

        "fg" => {
            // PATCH: Controllo se è stato passato un ID specifico (es: fg 2)
            let target_job = if let Some(arg) = args.first() {
                if let Ok(id) = arg.parse::<usize>() {
                    state::pop_job_by_id(id) // Estrae il job con quell'ID
                } else {
                    eprintln!("grsh: fg: {} non è un ID valido", arg);
                    return true;
                }
            } else {
                state::pop_last_job() // Comportamento di default: ultimo job
            };

            if let Some(job) = target_job {
                let pid = Pid::from_raw(job.pid);
                println!("{}", job.command);

                // 1. Cede il terminale al processo figlio
                let _ = unistd::tcsetpgrp(nix::libc::STDIN_FILENO, pid);
                
                // 2. Invia segnale di continuazione
                if let Err(e) = signal::kill(pid, Signal::SIGCONT) {
                    eprintln!("grsh: errore fg: {}", e);
                    return false;
                }

                // 3. Loop di attesa (permette nuovi Ctrl+Z)
                loop {
                    match waitpid(pid, Some(WaitPidFlag::WUNTRACED)) {
                        Ok(WaitStatus::Stopped(_, _)) => {
                            state::add_job(job.pid, job.command); 
                            break;
                        }
                        Ok(WaitStatus::Exited(_, _)) | Ok(WaitStatus::Signaled(_, _, _)) => break,
                        Err(_) => break,
                        _ => continue,
                    }
                }

                // 4. La shell si riprende il terminale
                let shell_pgid = unistd::getpgrp();
                let _ = unistd::tcsetpgrp(nix::libc::STDIN_FILENO, shell_pgid);
            } else {
                eprintln!("grsh: fg: job non trovato.");
            }
            true
        }

        "zap" => {
            let jobs = state::get_jobs();
            if jobs.is_empty() {
                println!("grsh: nulla da pulire.");
            } else {
                for j in jobs {
                    let _ = signal::kill(Pid::from_raw(j.pid), Signal::SIGKILL);
                    println!("[{}] {} terminato.", j.id, j.command);
                }
                state::clear_jobs();
            }
            true
        }

        // --- GUIDA E SESSIONE ---
        "help" => {
            print_help();
            true
        }

        "exit" | "quit" => {
            let jobs = state::get_jobs();
            if !jobs.is_empty() {
                println!("{}", Color::Yellow.bold().paint("Attenzione: ci sono job sospesi. Usa 'zap' o digita di nuovo exit."));
            }
            std::process::exit(0)
        }

        "version" => {
            println!("grsh (Grim Reaper SHell) version 0.1.0");
            println!("{}", Color::Red.bold().paint("Reaper is watching you! 😈"));
            true
        }

        // --- NAVIGAZIONE ---
        "cd" => {
            let home = env::var("HOME").unwrap_or_else(|_| "/".into());
            let target = args.get(0).copied().unwrap_or(&home);
            if let Err(e) = env::set_current_dir(target) {
                eprintln!("grsh: cd: {}: {}", target, e);
            }
            true
        }

        "pwd" => {
            if let Ok(cwd) = env::current_dir() { 
                println!("{}", cwd.display()); 
            }
            true
        }

        "mkcd" => {
            if let Some(dir) = args.first() {
                if let Err(e) = std::fs::create_dir_all(dir) {
                    eprintln!("grsh: mkcd: {}: {}", dir, e);
                } else { 
                    let _ = env::set_current_dir(dir); 
                }
            } else {
                eprintln!("grsh: mkcd: specificare una directory");
            }
            true
        }

        // --- AMBIENTE E CONFIGURAZIONE ---
        "reload" => {
            println!("{}", Color::Purple.bold().paint("Ricaricamento configurazione ~/.grshrc..."));
            let home = env::var("HOME").unwrap_or_else(|_| "/root".into());
            let rc_path = format!("{}/.grshrc", home);
            println!("Nota: Usa 'source {}' per applicare le modifiche.", rc_path);
            true
        }

        "setenv" => {
            if args.is_empty() {
                for (k, v) in env::vars() { println!("{}={}", k, v); }
            } else if args.len() == 1 {
                state::set_var(args[0], "");
            } else {
                state::set_var(args[0], args[1]);
            }
            true
        }

        "unsetenv" => {
            if let Some(key) = args.first() {
                env::remove_var(key);
                state::ENV_VARS.lock().unwrap().remove(*key);
            }
            true
        }

        "env" => {
            for (key, value) in env::vars() { 
                println!("{}={}", key, value); 
            }
            true
        }

        "alias" => {
            if args.is_empty() {
                for (n, v) in alias::get_all_aliases() { println!("alias {}='{}'", n, v); }
            } else {
                let full = args.join(" ");
                if let Some((n, v)) = full.split_once('=') {
                    alias::add_alias(n.trim(), v.trim().trim_matches('\'').trim_matches('\"'));
                }
            }
            true
        }

        // --- UTILITY ---
        "calc" => {
            if args.is_empty() {
                println!("Uso: calc \"(5+3)*2\"");
            } else {
                let expr = args.join(" ");
                match meval::eval_str(&expr) {
                    Ok(res) => println!("{} = {}", expr, Color::Green.bold().paint(res.to_string())),
                    Err(e) => eprintln!("grsh: calc: {}", e),
                }
            }
            true
        }

        "sysinfo" => {
            println!("{}", Color::Cyan.bold().paint("--- Reaper System Info ---"));
            println!("OS:           {} ({})", env::consts::OS, env::consts::ARCH);
            println!("Shell:       grsh v0.1.0");
            println!("Uptime:      {}s", SHELL_START_TIME.elapsed().as_secs());
            true
        }

        "which" => {
            if let Some(name) = args.first() {
                if let Some(path) = state::find_in_path(name) { println!("{}", path.display()); }
                else { eprintln!("{} non trovato", name); }
            }
            true
        }

        "type" => {
            if let Some(name) = args.first() {
                if alias::get_all_aliases().iter().any(|(n, _)| n == *name) { 
                    println!("{} è un alias", name); 
                }
                else if is_builtin(name) { println!("{} è un built-in di grsh", name); }
                else if let Some(path) = state::find_in_path(name) { println!("{} è {}", name, path.display()); }
            }
            true
        }

        "exec" => {
            if let Some(bin) = args.first() {
                let mut c = std::process::Command::new(bin);
                if args.len() > 1 { c.args(&args[1..]); }
                let _ = c.exec();
                eprintln!("grsh: exec fallito");
            }
            true
        }

        _ => false,
    }
}

fn is_builtin(name: &str) -> bool {
    let b = [
        "exit", "quit", "cd", "pwd", "mkcd", "calc", "sysinfo", 
        "which", "type", "setenv", "unsetenv", "env", "exec", 
        "version", "alias", "help", "reload", "jobs", "fg", "zap"
    ];
    b.contains(&name)
}

fn print_help() {
    println!("{}", Color::Yellow.bold().paint("╔════════════════════════════════════════════════════════════╗"));
    println!("{}", Color::Yellow.bold().paint("║            GRSH - GRIM REAPER SHELL HELP                   ║"));
    println!("{}", Color::Yellow.bold().paint("╚════════════════════════════════════════════════════════════╝"));
    
    println!("\n{}", Color::Cyan.bold().paint("--- Gestione Processi (Job Control) ---"));
    println!("  jobs             Elenca i processi sospesi");
    println!("  fg [id]          Riprende un processo specifico o l'ultimo");
    println!("  zap              Termina forzatamente tutti i processi sospesi");

    println!("\n{}", Color::Cyan.bold().paint("--- Navigazione & Filesystem ---"));
    println!("  cd <dir>         Cambia directory (default: HOME)");
    println!("  pwd              Mostra la directory corrente");
    println!("  mkcd <dir>       Crea una directory e vi accede");

    println!("\n{}", Color::Cyan.bold().paint("--- Ambiente & Configurazione ---"));
    println!("  setenv K V       Imposta una variabile d'ambiente");
    println!("  unsetenv K       Rimuove una variabile d'ambiente");
    println!("  env              Mostra tutte le variabili d'ambiente");
    println!("  alias N='C'      Crea un alias per un comando");
    println!("  source <file>    Esegue i comandi da un file");
    println!("  reload           Info su ricaricamento ~/.grshrc");
    println!("  exec <cmd>       Sostituisce la shell con un altro processo");

    println!("\n{}", Color::Cyan.bold().paint("--- Utility & Sistema ---"));
    println!("  calc <expr>      Calcolatrice (es: calc \"(5+3)*2\")");
    println!("  which <cmd>      Trova il percorso di un eseguibile");
    println!("  type <cmd>       Descrive la natura del comando");
    println!("  sysinfo          Info sistema e uptime shell");

    println!("\n{}", Color::Cyan.bold().paint("--- Sessione ---"));
    println!("  help             Mostra questa guida");
    println!("  exit / quit      Esce dalla shell");
    println!("");
}
