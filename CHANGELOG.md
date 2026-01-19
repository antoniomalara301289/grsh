# Changelog - GRSH (Grim Reaper SHell)

Tutti i cambiamenti degni di nota a questo progetto saranno documentati in questo file.

## [0.1.1] - 2026-01-19

### Fixed
- **Autocomplete**: Corretto il comportamento dei suggerimenti con percorsi contenenti spazi. Ora i nomi con spazi vengono automaticamente racchiusi tra virgolette (`" "`).
- **Terminal Control**: Migliorato il passaggio di proprietà del terminale (TTY) tra shell e processi figli.

### Changed
- Ottimizzato il parser degli argomenti per gestire correttamente le stringhe quotate e gli escape.
- Aggiornato il file `.gitignore` per escludere file di backup (`.bak`) e file di sistema macOS (`.DS_Store`).

---
## [0.1.0] - Versione Iniziale
- Supporto base per piping e redirezione.
- Integrazione AI tramite `tgpt`.
- Redirezione intelligente verso PDF.
- Alias e variabili d'ambiente.
