# aclgui

Cross-platform ACL & file permissions GUI for **Windows**, **macOS**, and **Linux**.

Built with pure Rust + [egui](https://github.com/emilk/egui) — no Electron, no Chromium, no WebView, no bundled runtime. Release binaries are **3–8 MB**.

[![npm](https://img.shields.io/npm/v/@eliekh05/aclgui)](https://www.npmjs.com/package/@eliekh05/aclgui)
[![Linux CI](https://github.com/eliekh05/aclgui/actions/workflows/Linux.yml/badge.svg)](https://github.com/eliekh05/aclgui/actions/workflows/Linux.yml)
[![macOS CI](https://github.com/eliekh05/aclgui/actions/workflows/macOS.yml/badge.svg)](https://github.com/eliekh05/aclgui/actions/workflows/macOS.yml)
[![Windows CI](https://github.com/eliekh05/aclgui/actions/workflows/Windows.yml/badge.svg)](https://github.com/eliekh05/aclgui/actions/workflows/Windows.yml)

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
| Raw output view | ✔ | ✔ | ✔ |
| OS detection + tool probing | ✔ | ✔ | ✔ |

---

## Install

### From npm (recommended — installs the right prebuilt binary for your platform)

```sh
# Run once without installing
npx @eliekh05/aclgui

# Or install globally
npm install -g @eliekh05/aclgui
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
git clone https://github.com/eliekh05/aclgui.git
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
5. **Apply** — click Apply. If the process is not elevated, it automatically relaunches with admin privileges and reopens the same path — no need to pick again.

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

> **Linux display requirement:** aclgui is a GUI application and requires a running X11 or Wayland display server. It will not start on headless servers. Set `DISPLAY` or `WAYLAND_DISPLAY` before running, or use a tool like `Xvfb` for testing.

---

## Notes

- **POSIX ACL mask**: The POSIX mask caps effective rights for named users and groups even when their ACE says Allow. The GUI shows the effective rights in the ACE table. To raise the mask, change it to match the highest rights you intend to grant.
- **Windows SYNCHRONIZE**: Denying certain rights via icacls implicitly adds SYNCHRONIZE, which blocks Explorer. aclgui warns you on DENY ACEs that would trigger this.
- **macOS SIP / TCC**: Apple-system paths under `/System`, `/usr`, and similar are protected at the kernel level and cannot be modified by any user-mode tool regardless of privilege.
- **NFSv4**: Requires `nfs4_getfacl` / `nfs4_setfacl` to be installed and the mount to support full NFSv4 ACLs.

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
│       ├── staged_panel.rs
│       └── raw_panel.rs
├── crates/
│   └── acl-core/           # Pure Rust library
│       └── src/
│           ├── lib.rs
│           ├── model.rs    # Unified ACL data model
│           ├── os_detect.rs
│           ├── parse.rs    # OS-native parsers
│           └── apply.rs    # OS-native appliers
├── npm/
│   └── aclgui/             # npm metapackage (bin-shim wrapper)
├── scripts/
│   └── npm-stamp.mjs       # Stamps + publishes per-platform npm packages
└── .github/
    └── workflows/
        ├── bump-version.yml  # Run manually: bumps version, commits, tags, triggers release
        ├── Linux.yml         # Linux CI
        ├── macOS.yml         # macOS CI
        ├── Windows.yml       # Windows CI
        └── release.yml       # Cross-compile + GitHub Release + npm publish (triggered by tag)
```

---

## License

[MIT](https://github.com/eliekh05/aclgui/blob/main/LICENSE)

## Acknowledgements

- Project author and maintainer: **@eliekh05**
- Initial development assisted by **Cursor AI**
- Additional edits and code review assisted by **Claude AI** (using the maintainer's own API credentials)
- Final implementation, testing, and releases are maintained by **@eliekh05**
