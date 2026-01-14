GRSH - Grim Reaper SHell (v0.1.0) 💀
GRSH è una shell Unix-like avanzata scritta in Rust. Non è solo un interprete di comandi, ma un ambiente di lavoro intelligente che integra automazione del workflow, intelligenza artificiale e un'interfaccia utente moderna.

🌟 Funzionalità Esclusive
🤖 Intelligenza Artificiale Integrata

GRSH supporta l'assistenza IA nativa tramite tgpt.

Basta iniziare un comando con ? per interrogare l'IA direttamente dalla shell.

Esempio: ? come posso comprimere una cartella in tar.gz?

📄 Smart Redirection (Auto-PDF)

La shell gestisce i redirect in modo intelligente in base all'estensione del file:

Standard: ls > output.txt crea un normale file di testo.

Smart: ls > output.pdf attiva automaticamente una pipeline interna che usa enscript e ps2pdf per generare un documento PDF formattato partendo dall'output del comando.

🔍 UI Moderna & Autocomplete

Tab-Completion Avanzato: Autocompletamento intelligente dei comandi e dei percorsi.

Menu Scorrevole: Navigazione dei suggerimenti tramite le frecce direzionali per una selezione rapida e intuitiva.

🚀 Caratteristiche Tecniche
Pipe & Redirect: Supporto completo a |, >, >>, e <.

Job Control: Gestione nativa dei processi con jobs, fg, e il comando zap per la pulizia totale.

Built-in Potenziati: calc (calcolatrice), mkcd (crea e entra), sysinfo (stato sistema), alias, e source.

Prompt Dinamico: Integrazione Git (branch e stato), indicatore job attivi e path abbreviato.

🛠️ Installazione e Dipendenze
Dipendenze di Sistema

Per il funzionamento delle feature avanzate, installa:

tgpt: Per il supporto AI (?).

enscript & ghostscript (ps2pdf): Per la generazione automatica dei PDF.

Compilazione
cargo build --release

⚙️ Configurazione Dinamica (~/.grshrc)
GRSH permette di usare i propri redirect per auto-configurarsi. Ecco come gestisce l'hostname dinamico nel tuo .grshrc:

# Setup dell'ambiente
setenv PATH /usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:/Users/antoniomalara/.cargo/bin
setenv EDITOR nano

# Bootstrap dinamico dell'hostname (Senza modificare il codice Rust)
echo -n "setenv HOSTNAME " > /tmp/load_host.grsh
hostname -s >> /tmp/load_host.grsh
source /tmp/load_host.grsh
rm /tmp/load_host.grsh

# Alias e Prompt
alias ls ls -G
if ($?prompt) then
    set prompt = "%{\033[1;31m%}%n%{\033[1;32m%}@%m%{\033[0m%}:%{\033[1;36m%}%~%{\033[0m%}%# "
endif

Configurazione rapida: Per usare la configurazione di default, copia il file d'esempio nella tua home: cp grshrc.example ~/.grshrc

📖 Tabella dei Comandi
Categoria	Comandi
AI	? <domanda>
Jobs	jobs, fg [id], zap
Filesystem	cd, pwd, mkcd
Redirection	>, >>, <, `
Utility	calc, which, sysinfo, alias, source

macOS (Homebrew)

🚀 Installazione
Puoi installare grsh su MacOS usando il mio tap:
brew tap antoniomalara301289/tap
brew install grsh

👨‍💻 Autore
Antonio Malara - Progetto GRSH v0.1.0
