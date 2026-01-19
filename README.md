# GRSH - Grim Reaper SHell (v0.1.1) 💀

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD--3--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)
[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)

**GRSH** is an advanced Unix-like shell written in **Rust**. It is designed as an intelligent workflow environment that integrates native automation, AI assistance, and a modern terminal UI.

Originally built with a **FreeBSD-first** philosophy, it works seamlessly on **macOS** and other BSD-based systems.

---

## 🌟 Key Features

### 🤖 Native AI Integration
GRSH features built-in AI assistance powered by `tgpt`.
* Start any command with `?` to query the AI directly from the shell.
* **Example:** `? how can I compress a folder to tar.gz?`

### 📄 Smart Redirection (Auto-PDF)
The shell handles output redirection based on the file extension:
* **Standard:** `ls > output.txt` creates a regular text file.
* **Smart:** `ls > output.pdf` automatically triggers an internal pipeline using `enscript` and `ps2pdf` to generate a formatted PDF document.

### 🔍 Modern UI & UX
* **Advanced Tab-Completion:** Intelligent suggestions for commands and file paths.
* **Interactive Scroll Menu:** Navigate suggestions using arrow keys.
* **Syntax Highlighting:** Real-time command coloring as you type.

---

## 🛠️ Installation & Setup

### 1. Prerequisites
To enable AI and PDF features, ensure these dependencies are installed:
* **[tgpt](https://github.com/a7ul/tgpt)** (for AI support)
* **enscript** & **ghostscript** (for PDF generation)

### 2. Install GRSH

**Via Homebrew (macOS):**
```bash
brew tap antoniomalara301289/tap
brew install grsh
```

**Via GIT (FreeBSD/Linux...)**
```bash
git clone [https://github.com/antoniomalara301289/grsh](https://github.com/antoniomalara301289/grsh)
cd grsh
cargo build --release
```

### 3. Configuration (~/.grshrc)
GRSH uses a .grshrc file for customization.
**Quick Start:**
```bash
cp grshrc.example ~/.grshrc
```

**Configuration Example (.grshrc):**
```bash
# Environment Setup
setenv PATH /usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$HOME/.cargo/bin
setenv EDITOR nano

# Dynamic Hostname Bootstrap (Example of GRSH scripting power)
echo -n "setenv HOSTNAME " > /tmp/load_host.grsh
hostname -s >> /tmp/load_host.grsh
source /tmp/load_host.grsh
rm /tmp/load_host.grsh

# Aliases & Visuals
alias ls ls -G
set prompt = "%{\033[1;31m%}%n%{\033[1;32m%}@%m%{\033[0m%}:%{\033[1;36m%}%~%{\033[0m%}%# "
```

### 🚀 Technical Specifications
• Pipe & Redirect: Full support for |, >, >>, and <.
• Job Control: Native management with jobs, fg, and the zap command.
• Enhanced Built-ins: calc, mkcd, sysinfo, alias, and source.
• Dynamic Prompt: Integrated Git status (branch/state) and job indicators.

### 📖 Command Reference

| Category | Commands |
| :--- | :--- |
| **AI** | `? <question>` |
| **Jobs** | `jobs`, `fg [id]`, `zap` |
| **Filesystem** | `cd`, `pwd`, `mkcd` |
| **Redirection** | `>`, `>>`, `<`, `|` |
| **Utility** | `calc`, `which`, `sysinfo`, `alias`, `source` |

### 📜 Changelog
Check the [CHANGELOG.md](CHANGELOG.md) for a detailed list of changes in every version.

### 👨‍💻 Author
Antonio Malara - Lead Developer
Project GRSH v0.1.1

