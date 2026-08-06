use std::path::Path;
use std::process::Command;

use crate::model::*;
use crate::os_detect::{current_os, Os, ToolAvailability};

pub type ApplyResult = Result<String, String>;

/// Apply a changeset. Runs the appropriate OS command(s).
/// Caller is responsible for ensuring elevation has been obtained.
pub fn apply_changes(cs: &ChangeSet, tools: &ToolAvailability) -> ApplyResult {
    let path = Path::new(&cs.path);
    let mut messages = Vec::new();

    for change in &cs.changes {
        let msg = apply_one(path, change, tools)?;
        messages.push(msg);
    }
    Ok(messages.join("\n"))
}

fn apply_one(path: &Path, change: &Change, _tools: &ToolAvailability) -> ApplyResult {
    match current_os() {
        Os::Windows => apply_windows(path, change),
        Os::Macos => apply_macos(path, change),
        Os::Linux => apply_linux(path, change),
        Os::Other => Err("Unsupported OS".into()),
    }
}

// ─── Linux ────────────────────────────────────────────────────────────────────

fn apply_linux(path: &Path, change: &Change) -> ApplyResult {
    match change {
        Change::SetMode { octal } => run(&["chmod", &format!("{octal:o}"), path_str(path)]),
        Change::SetOwner { user } => run(&["chown", user, path_str(path)]),
        Change::SetGroup { group } => run(&["chgrp", group, path_str(path)]),
        Change::RemoveAllAces => run(&["setfacl", "-b", "-k", path_str(path)]),
        Change::AddAce { ace, default } => {
            let spec = ace_to_setfacl_spec(ace, *default)?;
            run(&["setfacl", "-m", &spec, path_str(path)])
        }
        Change::RemoveAce { .. } | Change::ModifyAce { .. } => {
            Err("Re-read the path and re-build the full ACL spec to avoid index drift.".into())
        }
        Change::DisableInheritance { .. } | Change::EnableInheritance => {
            Err("POSIX ACLs do not have Windows-style inheritance; use default ACLs on parent directories.".into())
        }
    }
}

fn ace_to_setfacl_spec(ace: &Ace, default: bool) -> Result<String, String> {
    let prefix = if default { "default:" } else { "" };
    let kind_name = match &ace.principal {
        Principal::Owner => "user:".into(),
        Principal::OwningGroup => "group:".into(),
        Principal::Other => "other:".into(),
        Principal::Mask => "mask:".into(),
        Principal::User(n) => format!("user:{n}"),
        Principal::Group(n) => format!("group:{n}"),
        Principal::Everyone => "other:".into(),
        Principal::Sid(s) => return Err(format!("Cannot convert SID {s} to POSIX ACL")),
    };
    let r = ace.rights.summary();
    Ok(format!("{prefix}{kind_name}:{r}"))
}

// ─── macOS ────────────────────────────────────────────────────────────────────

fn apply_macos(path: &Path, change: &Change) -> ApplyResult {
    match change {
        Change::SetMode { octal } => run(&["chmod", &format!("{octal:o}"), path_str(path)]),
        Change::SetOwner { user } => run(&["chown", user, path_str(path)]),
        Change::SetGroup { group } => run(&["chgrp", group, path_str(path)]),
        Change::RemoveAllAces => run(&["chmod", "-N", path_str(path)]),
        Change::AddAce { ace, .. } => {
            let spec = ace_to_macos_chmod_spec(ace)?;
            run(&["chmod", "+a", &spec, path_str(path)])
        }
        Change::RemoveAce { index, .. } => {
            // chmod -a# N ... removes by index
            run(&["chmod", &format!("-a#{index}"), path_str(path)])
        }
        Change::ModifyAce { index, ace, .. } => {
            let spec = ace_to_macos_chmod_spec(ace)?;
            run(&["chmod", &format!("=a#{index}"), &spec, path_str(path)])
        }
        Change::DisableInheritance { .. } | Change::EnableInheritance => {
            Err("macOS uses ordered ACE lists; manage inheritance via file_inherit / directory_inherit flags.".into())
        }
    }
}

fn ace_to_macos_chmod_spec(ace: &Ace) -> Result<String, String> {
    let principal = match &ace.principal {
        Principal::User(n) => format!("user:{n}"),
        Principal::Group(n) => format!("group:{n}"),
        Principal::Everyone => "everyone".into(),
        other => return Err(format!("Cannot represent {other:?} as macOS ACE")),
    };
    let verb = if ace.allow { "allow" } else { "deny" };
    let mut perms = Vec::new();
    if ace.rights.read {
        perms.push("read");
    }
    if ace.rights.write {
        perms.push("write");
    }
    if ace.rights.execute {
        perms.push("execute");
    }
    if ace.rights.delete {
        perms.push("delete");
    }
    if ace.rights.append {
        perms.push("append");
    }
    if ace.rights.create_file {
        perms.push("add_file");
    }
    if ace.rights.create_dir {
        perms.push("add_subdirectory");
    }
    if ace.inherit.file_inherit {
        perms.push("file_inherit");
    }
    if ace.inherit.dir_inherit {
        perms.push("directory_inherit");
    }
    if perms.is_empty() {
        return Err("Cannot create an ACE with no rights selected.".into());
    }
    Ok(format!("{principal} {verb} {}", perms.join(",")))
}

// ─── Windows ──────────────────────────────────────────────────────────────────

fn apply_windows(path: &Path, change: &Change) -> ApplyResult {
    match change {
        Change::SetMode { .. } => Err("Windows does not use POSIX mode bits.".into()),
        Change::SetOwner { user } => run(&["icacls", path_str(path), "/setowner", user]),
        Change::SetGroup { .. } => {
            Err("Windows does not have a separate group owner concept.".into())
        }
        Change::RemoveAllAces => run(&["icacls", path_str(path), "/inheritance:r", "/reset"]),
        Change::AddAce { ace, .. } => {
            let (flag, principal, rights) = windows_ace_parts(ace)?;
            run(&[
                "icacls",
                path_str(path),
                flag,
                &format!("{principal}:{rights}"),
            ])
        }
        Change::RemoveAce { .. } | Change::ModifyAce { .. } => {
            Err("Rebuild and replace the ACL to avoid index drift on Windows.".into())
        }
        Change::DisableInheritance { copy_existing } => {
            let flag = if *copy_existing {
                "/inheritance:d"
            } else {
                "/inheritance:r"
            };
            run(&["icacls", path_str(path), flag])
        }
        Change::EnableInheritance => run(&["icacls", path_str(path), "/inheritance:e"]),
    }
}

fn windows_ace_parts(ace: &Ace) -> Result<(&'static str, String, String), String> {
    let flag = if ace.allow { "/grant" } else { "/deny" };
    let principal = match &ace.principal {
        Principal::User(n) => n.clone(),
        Principal::Group(n) => n.clone(),
        Principal::Sid(s) => s.clone(),
        Principal::Everyone => "Everyone".into(),
        other => return Err(format!("Cannot express {other:?} as Windows ACE")),
    };
    let mut rights = String::new();
    if ace.inherit.object_inherit {
        rights.push_str("(OI)");
    }
    if ace.inherit.container_inherit {
        rights.push_str("(CI)");
    }
    if ace.rights.read
        && ace.rights.write
        && ace.rights.execute
        && ace.rights.delete
        && ace.rights.write_security
    {
        rights.push('F');
    } else if ace.rights.read && ace.rights.write && ace.rights.execute && ace.rights.delete {
        rights.push_str("M");
    } else if ace.rights.read && ace.rights.execute {
        rights.push_str("RX");
    } else if ace.rights.read {
        rights.push('R');
    } else if ace.rights.write {
        rights.push('W');
    } else {
        // Build from individual bits
        let mut bits = Vec::new();
        if ace.rights.read {
            bits.push("RD");
        }
        if ace.rights.write {
            bits.push("WD");
        }
        if ace.rights.execute {
            bits.push("X");
        }
        if ace.rights.delete {
            bits.push("DE");
        }
        if ace.rights.read_attr {
            bits.push("RA");
        }
        if ace.rights.write_attr {
            bits.push("WA");
        }
        if ace.rights.read_security {
            bits.push("RC");
        }
        if ace.rights.write_security {
            bits.push("WDAC");
        }
        rights.push_str(&format!("({})", bits.join(",")));
    }
    Ok((flag, principal, rights))
}

// ─── helpers ──────────────────────────────────────────────────────────────────

fn path_str(p: &Path) -> &str {
    p.to_str().unwrap_or("")
}

fn run(args: &[&str]) -> ApplyResult {
    let (cmd, rest) = args.split_first().ok_or("empty command")?;
    let out = Command::new(cmd)
        .args(rest)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if out.status.success() {
        Ok(if stdout.is_empty() {
            "OK".into()
        } else {
            stdout
        })
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}
