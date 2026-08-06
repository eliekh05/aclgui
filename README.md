# aclgui

Cross-platform ACL & file permissions GUI for **Windows**, **macOS**, and **Linux**.

Built with pure Rust + [egui](https://github.com/emilk/egui) — no Electron, no Chromium, no WebView, no bundled runtime. Release binaries are **3–8 MB**.

---

## Features

| | Windows | macOS | Linux |
|---|---|---|---|
| POSIX mode bits (chmod) | — | ✔ | ✔ |
| POSIX.1e ACL (getfacl/setfacl) | — | — | ✔ |
| macOS NFSv4 ACL (ls -le / chmod +a) | — | ✔ | — |
| NFSv4 ACL (nfs4_getfacl/nfs4_setfacl) | — | ✔ (optional) | ✔ (optional) |
| Windows NTFS DACL (icacls) | ✔ | — | — |
| Staged changes (preview before apply) | ✔ | ✔ | ✔ |
| ACE editor (add / edit / remove entries) | ✔ | ✔ | ✔ |
| Inheritance controls | ✔ | ✔ | ✔ |
| Elevation prompt (UAC / pkexec / osascript) | ✔ | ✔ | ✔ |
| Interactive Help Chat | ✔ | ✔ | ✔ |
| Raw output view | ✔ | ✔ | ✔ |
| OS detection + tool probing | ✔ | ✔ | ✔ |

### Surprise: Interactive Help Chat

The built-in chat panel answers plain-English questions about the currently loaded path:

- *"Why can't alice write here?"* → inspects deny ACEs, POSIX mask, mode bits, Windows SYNCHRONIZE trap
- *"What does inherit mean?"* → OS-specific answer
- *"How do I add a group?"* → step-by-step for the current OS
- *"What is the mask?"* → explains POSIX ACL mask and how to fix it
- *"What is Synchronize on Windows?"* → explains the icacls/Explorer mismatch

No internet required. No LLM. Fully rule-based and always accurate to what is loaded.

---

## Install

### From npm (recommended — installs the right prebuilt binary for your platform)

```sh
npm -g @eliekh05/aclgui
aclgui
```

No postinstall scripts. npm installs only the matching platform sub-package via `optionalDependencies`. The shim launches the native binary directly.

### From GitHub Releases

Download the binary for your platform from [Releases](https://github.com/eliekh05/aclgui/releases) and run it directly. No installation required.

| Platform | File |
|---|---|
| macOS Apple Silicon | `aclgui-darwin-arm64` |
| macOS Intel | `aclgui-darwin-x64` |
| Linux x64 | `aclgui-linux-x64` |
| Linux ARM64 | `aclgui-linux-arm64` |
| Windows x64 | `aclgui-win32-x64.exe` |

### From source

```sh
git clone https://github.com/eliekhalil/aclgui.git
cd aclgui
cargo build --release
./target/release/aclgui
```

---

## Usage

1. **Pick a path** — type it in the top bar, press Enter, or use the File / Dir picker buttons.
2. **View permissions** — the Permissions tab shows POSIX mode bits and all ACE entries.
3. **Edit** — click **Add ACE** or the edit/delete buttons on existing entries. Changes are staged, not applied immediately.
4. **Review** — switch to the Staged tab to see a diff of pending changes.
5. **Apply** — click Apply. If the process is not elevated, it re-launches with admin privileges automatically (UAC on Windows, pkexec on Linux, osascript on macOS).
6. **Ask for help** — open the Help Chat tab and ask anything about the loaded path.

---

## Prerequisites

The GUI uses OS-native command-line tools. Most are pre-installed:

| Tool | Required for | Pre-installed? |
|---|---|---|
| `icacls` | Windows NTFS ACLs | ✔ Windows |
| `getfacl` / `setfacl` | Linux POSIX ACLs | ✔ most Linux distros |
| `ls`, `stat`, `chmod` | macOS / Linux basic perms | ✔ always |
| `nfs4_getfacl` / `nfs4_setfacl` | NFSv4 ACLs | install `nfs4-acl-tools` |
| `pkexec` | Linux elevation | ✔ most Linux distros |

---

## Known limitations

- **POSIX ACL mask**: The POSIX mask silently caps named user/group permissions. The GUI warns you when the mask restricts effective rights. Use the Help Chat for guidance.
- **Windows icacls SYNCHRONIZE trap**: icacls silently adds SYNCHRONIZE when denying certain rights, which locks out Explorer. The GUI uses the Win32 API where possible and warns on DENY ACEs that would imply this.
- **NFSv4**: Available as view + validated text roundtrip. The server must support full NFSv4 ACLs; not all NFS servers do.
- **macOS SIP / TCC**: System-protected paths cannot be edited by any user-mode tool.
- **No recursive apply yet**: Changes apply to the selected path only. Recursive support is planned.

---

## Repo structure

```
aclgui/
├── src/                    # egui GUI application
│   ├── main.rs
│   ├── app.rs              # App state
│   ├── elevation.rs        # Per-OS elevation
│   └── ui/                 # UI panels
│       ├── mod.rs
│       ├── top_bar.rs
│       ├── permissions_panel.rs
│       ├── ace_editor.rs
│       ├── chat_panel.rs
│       ├── staged_panel.rs
│       └── raw_panel.rs
├── crates/
│   └── acl-core/           # Pure Rust library
│       └── src/
│           ├── lib.rs
│           ├── model.rs    # Unified ACL data model
│           ├── os_detect.rs
│           ├── parse.rs    # OS-native parsers
│           ├── apply.rs    # OS-native appliers
│           └── chatbot.rs  # Rule-based help chat
├── npm/
│   └── aclgui/             # npm metapackage (bin-shim wrapper)
├── scripts/
│   └── npm-stamp.mjs       # Stamps + publishes per-platform npm packages
└── .github/
    └── workflows/
        ├── ci.yml          # PR checks on all 3 OSes
        └── release.yml     # Cross-compile + GitHub Release + npm publish
```

---

## License

MIT
