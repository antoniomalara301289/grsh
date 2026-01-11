use reedline::{Prompt, PromptHistorySearch, PromptHistorySearchStatus};
use nu_ansi_term::{Color, Style};
use std::borrow::Cow;
use std::env;

pub struct GrshPrompt {
    pub prompt_str: String,
}

impl Prompt for GrshPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        // Se il prompt_str è vuoto o di default, costruiamo quello richiesto
        if self.prompt_str.is_empty() {
            let user = env::var("USER").unwrap_or_else(|_| "user".into());
            let host = "server"; // Potresti usare gethostname crate
            let pwd = env::current_dir()
                .unwrap_or_else(|_| "/".into())
                .display()
                .to_string();

            let user_color = if user == "root" { Color::Red } else { Color::Green };
            let at_color = Color::Green;
            let pwd_color = Color::Blue;

            let res = format!(
                "{}{}{} {} ➜ ",
                user_color.bold().paint(user),
                at_color.paint("@"),
                at_color.paint(host),
                pwd_color.bold().paint(pwd),
            );
            return Cow::Owned(res);
        }
        
        // Qui andrebbe la logica per interpretare variabili tipo \u, \h, \w dal .grshrc
        Cow::Owned(self.prompt_str.clone())
    }

    fn render_prompt_right(&self) -> Cow<str> { Cow::Borrowed("") }
    fn render_prompt_indicator(&self, _edit_mode: reedline::EditMode) -> Cow<str> { Cow::Borrowed("") }
    fn render_prompt_multiline_indicator(&self) -> Cow<str> { Cow::Borrowed("::: ") }
    fn render_history_search_indicator(&self, _history_search: PromptHistorySearch) -> Cow<str> { Cow::Borrowed("search: ") }
}
