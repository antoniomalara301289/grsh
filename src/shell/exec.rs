use std::process::{Command, Stdio, Child};
use std::fs::{OpenOptions, File};
use crate::shell::{alias, state}; // Aggiunto state qui
use glob::glob;
use std::os::unix::process::CommandExt;

use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::unistd::{self, Pid};
use nix::sys::wait::{waitpid, WaitStatus, WaitPidFlag};

// ... smart_split rimane identico ...
fn smart_split(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;
    for c in line.chars() {
        match c {
            '"' if !in_single_quotes => in_double_quotes = !in_double_quotes,
            '\'' if !in_double_quotes => in_single_quotes = !in_single_quotes,
            ' ' if !in_double_quotes && !in_single_quotes => {
                if !current.is_empty() { args.push(current.clone()); current.clear(); }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() { args.push(current); }
    args
}

pub fn execute(line: &str) -> bool {
    if line.contains(" && ") {
        let parts: Vec<&str> = line.splitn(2, " && ").collect();
        return if execute(parts[0]) { execute(parts[1]) } else { false };
    }

    let commands_chunks: Vec<&str> = line.split('|').collect();
    let mut previous_child: Option<Child> = None;
    let mut last_status = true;

    for (i, cmd_chunk) in commands_chunks.iter().enumerate() {
        let is_last = i == commands_chunks.len() - 1;
        let mut current_chunk = cmd_chunk.trim().to_string();
        let mut file_out_redirect: Option<(String, bool)> = None;
        let mut file_in_redirect: Option<String> = None;

        if current_chunk.contains('<') {
            let parts: Vec<&str> = current_chunk.split('<').collect();
            if parts.len() == 2 {
                file_in_redirect = Some(parts[1].trim().to_string());
                current_chunk = parts[0].trim().to_string();
            }
        }

        if is_last {
            if current_chunk.contains(">>") {
                let parts: Vec<&str> = current_chunk.split(">>").collect();
                if parts.len() == 2 {
                    file_out_redirect = Some((parts[1].trim().to_string(), true));
                    current_chunk = parts[0].trim().to_string();
                }
            } else if current_chunk.contains('>') {
                let parts: Vec<&str> = current_chunk.split('>').collect();
                if parts.len() == 2 {
                    file_out_redirect = Some((parts[1].trim().to_string(), false));
                    current_chunk = parts[0].trim().to_string();
                }
            }
        }

        let expanded_chunk = resolve_single_command_alias(&current_chunk);
        let raw_parts = smart_split(&expanded_chunk);
        if raw_parts.is_empty() { continue; }
        
        let parts = expand_globs(raw_parts.iter().map(|s| s.as_str()).collect());
        let program = &parts[0];
        let args = &parts[1..];

        if let Some((ref filename, _)) = file_out_redirect {
            if filename.to_lowercase().ends_with(".pdf") {
                return execute_as_pdf(&parts, filename, previous_child);
            }
        }

        let stdin = if let Some(ref in_file) = file_in_redirect {
            if let Ok(f) = File::open(in_file) {
                Stdio::from(f)
            } else {
                eprintln!("grsh: impossibile aprire file input: {}", in_file);
                Stdio::null()
            }
        } else {
            previous_child.take().map_or(Stdio::inherit(), |child| {
                Stdio::from(child.stdout.expect("Failed to open stdout"))
            })
        };

        let stdout;
        let stderr;

        if let Some((ref filename, append)) = file_out_redirect {
            let file_result = OpenOptions::new()
                .create(true).write(true).append(append).truncate(!append)
                .open(filename);
            
            match file_result {
                Ok(f) => {
                    let f2 = f.try_clone().expect("Clone file handle failed");
                    stdout = Stdio::from(f);
                    stderr = Stdio::from(f2);
                },
                Err(e) => {
                    eprintln!("grsh: errore file: {}", e);
                    stdout = Stdio::inherit();
                    stderr = Stdio::inherit();
                }
            }
        } else if !is_last {
            stdout = Stdio::piped();
            stderr = Stdio::piped(); 
        } else {
            stdout = Stdio::inherit();
            stderr = Stdio::inherit();
        };

        let mut cmd = Command::new(program);
        cmd.args(args).stdin(stdin).stdout(stdout).stderr(stderr);

        unsafe {
            cmd.pre_exec(|| {
                let default_action = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
                let _ = sigaction(Signal::SIGINT, &default_action);
                let _ = sigaction(Signal::SIGTSTP, &default_action);
                let _ = sigaction(Signal::SIGQUIT, &default_action);
                let _ = sigaction(Signal::SIGTTOU, &default_action);

                let pid = unistd::getpid();
                let _ = unistd::setpgid(pid, pid);
                Ok(())
            });
        }

        match cmd.spawn() {
            Ok(mut child) => {
                let child_pid = Pid::from_raw(child.id() as i32);

                if is_last {
                    let _ = unistd::tcsetpgrp(nix::libc::STDIN_FILENO, child_pid);
                    
                    loop {
                        match waitpid(child_pid, Some(WaitPidFlag::WUNTRACED)) {
                            Ok(WaitStatus::Stopped(_, _)) => {
                                // --- PATCH: AGGIUNTA AL JOB CONTROL ---
                                // Salviamo il comando completo o solo il nome del programma
                                let full_command = parts.join(" ");
                                state::add_job(child_pid.as_raw(), full_command);
                                // Non impostiamo last_status perché il processo non è finito
                                break;
                            }
                            Ok(WaitStatus::Exited(_, status)) => {
                                last_status = status == 0;
                                break;
                            }
                            Ok(WaitStatus::Signaled(_, _, _)) => {
                                last_status = false;
                                break;
                            }
                            Err(_) => {
                                last_status = false;
                                break;
                            }
                            _ => continue,
                        }
                    }
                    
                    let shell_pgid = unistd::getpgrp();
                    let _ = unistd::tcsetpgrp(nix::libc::STDIN_FILENO, shell_pgid);
                    
                } else {
                    previous_child = Some(child);
                }
            }
            Err(_) => {
                eprintln!("grsh: command not found: {}", program);
                previous_child = None;
                last_status = false;
                break;
            }
        }
    }
    last_status
}

// ... Resto del file (expand_globs, execute_as_pdf, resolve_single_command_alias) rimane invariato ...
fn expand_globs(parts: Vec<&str>) -> Vec<String> {
    let mut expanded = Vec::new();
    for part in parts {
        if (part.contains('*') || part.contains('?')) && !part.starts_with('\'') {
            if let Ok(paths) = glob(part) {
                let mut found = false;
                for entry in paths.filter_map(Result::ok) {
                    expanded.push(entry.display().to_string());
                    found = true;
                }
                if !found { expanded.push(part.to_string()); }
            } else { expanded.push(part.to_string()); }
        } else {
            expanded.push(part.trim_matches('\'').trim_matches('"').to_string());
        }
    }
    expanded
}

fn execute_as_pdf(parts: &[String], output_pdf: &str, prev_child: Option<Child>) -> bool {
    let stdin = prev_child.map_or(Stdio::inherit(), |child| {
        Stdio::from(child.stdout.expect("Pipe error"))
    });

    if let Ok(child1) = Command::new(&parts[0]).args(&parts[1..]).stdin(stdin).stdout(Stdio::piped()).spawn() {
        if let Ok(child2) = Command::new("enscript").args(["-p", "-", "-q"]).stdin(Stdio::from(child1.stdout.unwrap())).stdout(Stdio::piped()).spawn() {
            if let Ok(mut child3) = Command::new("ps2pdf").args(["-", output_pdf]).stdin(Stdio::from(child2.stdout.unwrap())).spawn() {
                let _ = child3.wait();
                println!("grsh: PDF generato con successo: {}", output_pdf);
                return true;
            }
        }
    }
    eprintln!("grsh: errore PDF. Verifica enscript e ps2pdf.");
    false
}

fn resolve_single_command_alias(cmd_part: &str) -> String {
    let parts: Vec<&str> = cmd_part.split_whitespace().collect();
    if parts.is_empty() { return cmd_part.to_string(); }
    let resolved = alias::resolve_alias(parts[0]);
    if resolved != parts[0] {
        let clean_resolved = resolved.trim_matches('"').trim_matches('\'');
        if parts.len() > 1 { format!("{} {}", clean_resolved, parts[1..].join(" ")) } else { clean_resolved.to_string() }
    } else {
        cmd_part.to_string()
    }
}
