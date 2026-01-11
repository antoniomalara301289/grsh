use reedline::{Highlighter, StyledText};
use nu_ansi_term::{Color, Style};
use std::path::Path;

pub struct GrshHighlighter {
    pub commands: Vec<String>,
}

impl Highlighter for GrshHighlighter {
    fn highlight(&self, line: &str, _pos: usize) -> StyledText {
        let mut styled_text = StyledText::new();
        if line.is_empty() { return styled_text; }

        let words = line.split_inclusive(' ');

        for (i, word) in words.enumerate() {
            let trimmed = word.trim_end();
            let space_suffix = &word[trimmed.len()..];

            // 1. COMANDO PRINCIPALE
            if i == 0 {
                let exists = self.commands.contains(&trimmed.to_string()) 
                             || trimmed.contains('/') 
                             || trimmed.starts_with('.');
                let style = if exists { Color::Cyan } else { Color::Red };
                styled_text.push((Style::new().fg(style).bold(), trimmed.to_string()));
                styled_text.push((Style::new(), space_suffix.to_string()));
                continue;
            }

            // 2. FLAG
            if trimmed.starts_with('-') {
                styled_text.push((Style::new().fg(Color::Yellow), trimmed.to_string()));
                styled_text.push((Style::new(), space_suffix.to_string()));
                continue;
            }

            // 3. LOGICA PERCORSI (L'ANIMA DELLA TUA RICHIESTA)
            if trimmed.contains('/') || trimmed.starts_with('.') || trimmed.starts_with('~') {
                let path = Path::new(trimmed);
                
                if path.exists() {
                    if path.is_dir() {
                        // È tutto una cartella
                        styled_text.push((Style::new().fg(Color::Blue).bold(), trimmed.to_string()));
                    } else {
                        // È un file (o un link a un file): separiamo l'ultima parte
                        if let Some(slash_pos) = trimmed.rfind('/') {
                            let folder_part = &trimmed[..=slash_pos];
                            let file_part = &trimmed[slash_pos + 1..];
                            styled_text.push((Style::new().fg(Color::Blue).bold(), folder_part.to_string()));
                            styled_text.push((Style::new().fg(Color::White), file_part.to_string()));
                        } else {
                            styled_text.push((Style::new().fg(Color::White), trimmed.to_string()));
                        }
                    }
                } else {
                    // Il percorso non esiste ancora (mentre scrivi) o è testo
                    styled_text.push((Style::new().fg(Color::Fixed(250)), trimmed.to_string()));
                }
            } else {
                // Testo normale
                styled_text.push((Style::new().fg(Color::White), trimmed.to_string()));
            }

            // Aggiungi lo spazio finale
            styled_text.push((Style::new(), space_suffix.to_string()));
        }

        styled_text
    }
}
