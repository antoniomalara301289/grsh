use reedline::{
    ColumnarMenu, Emacs,
    KeyCode, KeyModifiers, Reedline, ReedlineEvent, ReedlineMenu,
    Signal, FileBackedHistory, MenuBuilder,
    Completer, Suggestion, Span, Hinter, History,
    SearchQuery, SearchDirection, SearchFilter, CommandLineSearch,
    CursorConfig,
};
use nu_ansi_term::Color;
use std::path::PathBuf;
use std::io::{self, Write};
use std::process::Command;

const NO_ARGS_CMDS: &[&str] = &["exit", "quit", "clear", "top", "htop", "pwd", "sysinfo", "version", "reload", "help", "env"];

//
// ---------------- LEVENSHTEIN ----------------
//
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0; b.len() + 1]; a.len() + 1];

    for i in 0..=a.len() { dp[i][0] = i; }
    for j in 0..=b.len() { dp[0][j] = j; }

    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

//
// ---------------- HINTER ----------------
//
struct GrshHinter {
    all_commands: Vec<String>,
    current_hint: String,
}

impl Hinter for GrshHinter {
    fn handle(
        &mut self,
        line: &str,
        _pos: usize,
        history: &dyn History,
        _use_completions: bool,
        _marker: &str,
    ) -> String {
        self.current_hint.clear();

        if line.trim().is_empty() {
            return String::new();
        }

        let first = line.split_whitespace().next().unwrap_or("");
        if NO_ARGS_CMDS.contains(&first) {
            return String::new();
        }

        let hint = get_raw_hint(line, history, &self.all_commands);
        if hint.is_empty() {
            return String::new();
        }

        self.current_hint = hint.clone();
        Color::Green.paint(hint).to_string()
    }

    fn complete_hint(&self) -> String {
        self.current_hint.clone()
    }

    fn next_hint_token(&self) -> String {
        String::new()
    }
}

fn get_raw_hint(
    line: &str,
    history: &dyn History,
    all_commands: &[String],
) -> String {
    let last_sep = line
        .rfind(|c: char| c == ' ' || c == '|')
        .map(|i| i + 1)
        .unwrap_or(0);

    let current_word = &line[last_sep..];

    if line.contains(' ') || current_word.contains('/') || current_word.starts_with('.') {
        let expanded = current_word.replacen(
            '~',
            &dirs::home_dir().unwrap_or_default().to_string_lossy(),
            1,
        );

        let path = std::path::Path::new(&expanded);
        let (dir, prefix) = if expanded.ends_with('/') {
            (path, "")
        } else {
            (
                path.parent().unwrap_or(std::path::Path::new(".")),
                path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            )
        };

        let dir_to_read = if dir.as_os_str().is_empty() { std::path::Path::new(".") } else { dir };

        if let Ok(entries) = std::fs::read_dir(dir_to_read) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(prefix) && name != prefix {
                    return name[prefix.len()..].to_string();
                }
            }
        }
        return String::new();
    }

    let query = SearchQuery {
        direction: SearchDirection::Backward,
        start_time: None, end_time: None, start_id: None, end_id: None,
        filter: SearchFilter::from_text_search(
            CommandLineSearch::Prefix(line.to_string()),
            None,
        ),
        limit: Some(1),
    };

    if let Ok(results) = history.search(query) {
        if let Some(entry) = results.first() {
            if let Some(hist_first) = entry.command_line.split_whitespace().next() {
                if all_commands.contains(&hist_first.to_string()) && hist_first.starts_with(line) {
                    return hist_first[line.len()..].to_string();
                }
            }
        }
    }

    String::new()
}

//
// ---------------- COMPLETER ----------------
//
struct GrshCompleter {
    base_commands: Vec<String>,
}

impl Completer for GrshCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let last_sep = line[..pos]
            .rfind(|c: char| c == ' ' || c == '|')
            .map(|i| i + 1)
            .unwrap_or(0);

        let current = &line[last_sep..pos];
        let mut out = Vec::new();

        if last_sep == 0 && !current.contains('/') && !current.starts_with('.') {
            for cmd in &self.base_commands {
                if cmd.starts_with(current) {
                    out.push(Suggestion {
                        value: cmd.clone(),
                        span: Span::new(last_sep, pos),
                        append_whitespace: true,
                        ..Default::default()
                    });
                }
            }
            return out;
        }

        let expanded = current.replacen(
            '~',
            &dirs::home_dir().unwrap_or_default().to_string_lossy(),
            1,
        );

        let path = std::path::Path::new(&expanded);
        
        let (dir, prefix) = if expanded.ends_with('/') {
            (path, "")
        } else {
            let p = path.parent().unwrap_or(std::path::Path::new("."));
            let f = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.as_os_str().is_empty() {
                (std::path::Path::new("."), f)
            } else {
                (p, f)
            }
        };

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                
                if name.starts_with(prefix) {
                    let is_dir = entry.path().is_dir();
                    
                    let mut value = if dir == std::path::Path::new(".") && !current.starts_with("./") && !current.starts_with('/') {
                        name
                    } else {
                        let mut base = dir.to_string_lossy().to_string();
                        if !base.ends_with('/') { base.push('/'); }
                        format!("{}{}", base, name)
                    };

                    if is_dir {
                        value.push('/');
                    }

                    out.push(Suggestion {
                        value,
                        span: Span::new(last_sep, pos),
                        append_whitespace: !is_dir,
                        ..Default::default()
                    });
                }
            }
        }

        out
    }
}

//
// ---------------- REPL LOOP ----------------
//
pub fn repl_loop(runner: fn(String) -> bool) {
    let mut history_path = dirs::home_dir().unwrap_or(PathBuf::from("."));
    history_path.push(".grsh_history");

    let history = Box::new(
        FileBackedHistory::with_file(1000, history_path)
            .expect("History error"),
    );

    let mut all_commands = vec!["exit".into(), "quit".into(), "clear".into()];
    if let Ok(path) = std::env::var("PATH") {
        for p in std::env::split_paths(&path) {
            if let Ok(entries) = std::fs::read_dir(p) {
                for e in entries.flatten() {
                    all_commands.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    let mut builtins_list = vec![
        "setenv".into(), "set".into(), "alias".into(), 
        "source".into(), "echo".into(), "if".into(), "endif".into(),
        "mkcd".into(), "calc".into(), "sysinfo".into(), // <--- Nuovi
        "reload".into(), "help".into(), "type".into(),   // <--- Nuovi
        "jobs".into(), "fg".into(), "zap".into(), // <--- AGGIUNTI QUI
    ];
    all_commands.append(&mut builtins_list);
    all_commands.sort();
    all_commands.dedup();

    let mut keybindings = reedline::default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('g'),
        ReedlineEvent::HistoryHintComplete,
    );
    
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let menu = ColumnarMenu::default()
        .with_name("completion_menu")
        .with_columns(4)
        .with_marker("");

    let highlighter = Box::new(crate::shell::syntax::GrshHighlighter {
        commands: all_commands.clone(),
    });

    // Inizializzazione editor con CursorConfig di default
    let mut editor = Reedline::create()
        .with_history(history)
        .with_completer(Box::new(GrshCompleter {
            base_commands: all_commands.clone(),
        }))
        .with_hinter(Box::new(GrshHinter {
            all_commands: all_commands.clone(),
            current_hint: String::new(),
        }))
        .with_highlighter(highlighter)
        .with_menu(ReedlineMenu::EngineCompleter(Box::new(menu)))
        .with_edit_mode(Box::new(Emacs::new(keybindings)))
        .with_cursor_config(CursorConfig::default());
    // --- PATCH: IGNORA CTRL+Z NELLA SHELL PADRE ---
    // Questo evita che la shell si chiuda/sospenda se premi Ctrl+Z a vuoto sul prompt
    unsafe {
        use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
        let ignore_action = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
        let _ = sigaction(Signal::SIGTSTP, &ignore_action);
    }
    let prompt = crate::config::grshrc::GrshPrompt;

    loop {
        match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                let cleaned = line.trim().to_string();
                if cleaned.is_empty() { continue; }

                let first = cleaned.split_whitespace().next().unwrap_or("");

                // Se il comando è conosciuto (sta in all_commands) lo eseguiamo subito
                if all_commands.contains(&first.to_string()) 
                    || first.contains('/') 
                    || first.starts_with('?') 
                    || first.starts_with('#') 
                {
                    if cleaned == "exit" || cleaned == "quit" { break; }
                    
                    if cleaned.starts_with('?') {
                        let q = cleaned[1..].trim();
                        if !q.is_empty() {
                            let _ = Command::new("tgpt").arg("-s").arg(q).status();
                        }
                        continue;
                    }

                    runner(cleaned); // QUI esegue i tuoi nuovi comandi!
                    continue;
                }

                // --- Se arriviamo qui, il comando è SCONOSCIUTO ---
                // Cancelliamo l'errore dalla cronologia
                let del_query = SearchQuery {
                    direction: SearchDirection::Backward,
                    start_time: None, end_time: None, start_id: None, end_id: None,
                    filter: SearchFilter::from_text_search(CommandLineSearch::Prefix("".to_string()), None),
                    limit: Some(1),
                };
                if let Ok(results) = editor.history_mut().search(del_query) {
                    if let Some(entry) = results.first() {
                        if let Some(id) = entry.id {
                            let _ = editor.history_mut().delete(id);
                            let _ = editor.history_mut().sync();
                        }
                    }
                }

                // Suggerimento Levenshtein (il "Forse volevi...")
                let mut best: Option<(String, usize)> = None;
                for cmd in &all_commands {
                    let d = levenshtein(first, cmd);
                    if d > 0 && d <= 2 {
                        if best.as_ref().map_or(true, |(_, old)| d < *old) {
                            best = Some((cmd.clone(), d));
                        }
                    }
                }

                if let Some((corr, _)) = best {
                    print!("grsh: comando '{}' non trovato. Forse volevi '{}'? [y/N]: ", first, corr);
                    std::io::stdout().flush().ok();
                    let mut resp = String::new();
                    std::io::stdin().read_line(&mut resp).ok();
                    if resp.trim().eq_ignore_ascii_case("y") {
                        runner(cleaned.replacen(first, &corr, 1));
                    }
                } else {
                    println!("grsh: comando '{}' non trovato.", first);
                }

                if cleaned == "exit" || cleaned == "quit" {
                    break;
                }

                if cleaned.starts_with('?') {
                    let q = cleaned[1..].trim();
                    if !q.is_empty() {
                        let _ = Command::new("tgpt").arg("-s").arg(q).status();
                    }
                    continue;
                }

                runner(cleaned);
            }

            Ok(Signal::CtrlC) => println!("^C"),
            Ok(Signal::CtrlD) => break,
            Err(_) => break,
        }
    }
}
