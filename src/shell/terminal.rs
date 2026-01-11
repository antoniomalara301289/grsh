use std::io::{self, Write};

pub fn set_cursor_style(style_code: &str) {
    // La sequenza è \x1b[X q dove X è il codice stile
    // Se lo stile è vuoto o non valido, non facciamo nulla
    if style_code.is_empty() { return; }
    
    print!("\x1b[{} q", style_code);
    let _ = io::stdout().flush();
}
