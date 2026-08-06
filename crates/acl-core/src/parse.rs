use std::path::Path;
use std::process::Command;

use crate::model::*;
use crate::os_detect::{current_os, path_has_nfs4_acl, Os, ToolAvailability};

/// Parse the ACL/permissions for a given path.
pub fn read_path(path: &Path, tools: &ToolAvailability) -> PathAcl {
    let mut acl = PathAcl::empty(path.to_string_lossy().to_string());
    acl.is_dir = path.is_dir();

    match current_os() {
        Os::Windows => read_windows(path, &mut acl),
        Os::Macos => {
            if path_has_nfs4_acl(path, tools) {
                read_nfs4(path, &mut acl);
            } else {
                read_macos(path, &mut acl);
            }
        }
        Os::Linux => {
            if path_has_nfs4_acl(path, tools) {
                read_nfs4(path, &mut acl);
            } else if tools.getfacl {
                read_posix_acl(path, &mut acl);
            } else {
                read_posix_mode_only(path, &mut acl);
            }
        }
        Os::Other => {
            read_posix_mode_only(path, &mut acl);
        }
    }
    acl
}

// ─── POSIX mode (stat) ────────────────────────────────────────────────────────

fn read_posix_mode_only(path: &Path, acl: &mut PathAcl) {
    acl.kind = AclKind::PosixMode;
    let out = Command::new("stat")
        .args(["-c", "%a %U %G", path.to_str().unwrap_or("")])
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            acl.error = Some(e.to_string());
            return;
        }
    };
    let s = String::from_utf8_lossy(&out.stdout);
    acl.raw_output = s.to_string();
    let parts: Vec<&str> = s.trim().split_whitespace().collect();
    if let Some(octal_str) = parts.first() {
        if let Ok(octal) = u32::from_str_radix(octal_str, 8) {
            acl.posix_mode = Some(PosixMode::from_octal(octal));
        }
    }
    if parts.len() > 1 {
        acl.owner = Some(parts[1].into());
    }
    if parts.len() > 2 {
        acl.group = Some(parts[2].into());
    }
}

// ─── POSIX ACL (getfacl) ──────────────────────────────────────────────────────

fn read_posix_acl(path: &Path, acl: &mut PathAcl) {
    acl.kind = AclKind::PosixAcl;
    let out = Command::new("getfacl")
        .args(["--absolute-names", "--", path.to_str().unwrap_or("")])
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            acl.error = Some(e.to_string());
            return;
        }
    };
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    acl.raw_output = text.clone();
    parse_getfacl_output(&text, acl);
}

fn parse_getfacl_output(text: &str, acl: &mut PathAcl) {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            if let Some(val) = line.strip_prefix("# owner: ") {
                acl.owner = Some(val.trim().into());
            } else if let Some(val) = line.strip_prefix("# group: ") {
                acl.group = Some(val.trim().into());
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }

        let is_default = line.starts_with("default:");
        let entry = if is_default {
            &line["default:".len()..]
        } else {
            line
        };

        let parts: Vec<&str> = entry.splitn(3, ':').collect();
        if parts.len() < 2 {
            continue;
        }

        let (kind, name, perms_str) = match parts.len() {
            3 => (parts[0], parts[1], parts[2]),
            2 => (parts[0], "", parts[1]),
            _ => continue,
        };

        let principal = match kind {
            "user" if name.is_empty() => Principal::Owner,
            "user" => Principal::User(name.into()),
            "group" if name.is_empty() => Principal::OwningGroup,
            "group" => Principal::Group(name.into()),
            "mask" => Principal::Mask,
            "other" => Principal::Other,
            _ => continue,
        };

        let perms_str = perms_str.split_whitespace().next().unwrap_or(perms_str);
        let rights = parse_rwx(perms_str);

        let ace = Ace {
            principal,
            allow: true,
            rights,
            inherit: InheritFlags::default(),
            is_default,
        };

        if is_default {
            acl.default_aces.push(ace);
        } else {
            acl.aces.push(ace);
        }
    }

    // Extract POSIX mode from owner/group/other entries
    build_posix_mode_from_aces(acl);
}

fn parse_rwx(s: &str) -> Rights {
    let chars: Vec<char> = s.chars().collect();
    Rights {
        read: chars.first().copied().unwrap_or('-') == 'r',
        write: chars.get(1).copied().unwrap_or('-') == 'w',
        execute: chars.get(2).copied().unwrap_or('-') == 'x',
        ..Default::default()
    }
}

fn build_posix_mode_from_aces(acl: &mut PathAcl) {
    let mut mode = PosixMode::default();
    for ace in &acl.aces {
        match &ace.principal {
            Principal::Owner => {
                mode.owner_read = ace.rights.read;
                mode.owner_write = ace.rights.write;
                mode.owner_execute = ace.rights.execute;
            }
            Principal::OwningGroup => {
                mode.group_read = ace.rights.read;
                mode.group_write = ace.rights.write;
                mode.group_execute = ace.rights.execute;
            }
            Principal::Other => {
                mode.other_read = ace.rights.read;
                mode.other_write = ace.rights.write;
                mode.other_execute = ace.rights.execute;
            }
            _ => {}
        }
    }
    acl.posix_mode = Some(mode);
}

// ─── macOS (ls -le) ───────────────────────────────────────────────────────────

fn read_macos(path: &Path, acl: &mut PathAcl) {
    acl.kind = AclKind::MacosAcl;

    // Get owner/group/mode
    let stat_out = Command::new("stat")
        .args(["-f", "%p %Su %Sg", path.to_str().unwrap_or("")])
        .output();
    if let Ok(o) = stat_out {
        let s = String::from_utf8_lossy(&o.stdout);
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        if let Some(octal_str) = parts.first() {
            if let Ok(full) = octal_str.parse::<u32>() {
                acl.posix_mode = Some(PosixMode::from_octal(full & 0o7777));
            }
        }
        if parts.len() > 1 {
            acl.owner = Some(parts[1].into());
        }
        if parts.len() > 2 {
            acl.group = Some(parts[2].into());
        }
    }

    // Get ACL entries
    let out = Command::new("ls")
        .args(["-led", path.to_str().unwrap_or("")])
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            acl.error = Some(e.to_string());
            return;
        }
    };
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    acl.raw_output = text.clone();
    parse_ls_le_output(&text, acl);
}

fn parse_ls_le_output(text: &str, acl: &mut PathAcl) {
    // Lines after the first are ACE entries:
    // " 0: user:alice allow add_file,delete,file_inherit,directory_inherit"
    // " 0: user:alice inherited allow read"
    for line in text.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Strip leading index like "0: "
        let entry = if let Some(pos) = line.find(':') {
            line[pos + 1..].trim()
        } else {
            line
        };

        // Split on whitespace: principal [inherited] allow|deny perms
        let tokens: Vec<&str> = entry.split_whitespace().collect();
        if tokens.len() < 3 {
            continue;
        }

        let mut idx = 0;
        let principal_str = tokens[idx];
        idx += 1;
        let inherited = if tokens.get(idx).copied() == Some("inherited") {
            idx += 1;
            true
        } else {
            false
        };
        let allow = match tokens.get(idx) {
            Some(&"allow") => {
                idx += 1;
                true
            }
            Some(&"deny") => {
                idx += 1;
                false
            }
            _ => continue,
        };
        let perms_str = tokens[idx..].join(",");

        let principal = parse_macos_principal(principal_str);
        let rights = parse_macos_perms(&perms_str);

        acl.aces.push(Ace {
            principal,
            allow,
            rights,
            inherit: InheritFlags {
                inherited,
                file_inherit: perms_str.contains("file_inherit"),
                dir_inherit: perms_str.contains("directory_inherit"),
                ..Default::default()
            },
            is_default: false,
        });
    }
}

fn parse_macos_principal(s: &str) -> Principal {
    if let Some(name) = s.strip_prefix("user:") {
        Principal::User(name.into())
    } else if let Some(name) = s.strip_prefix("group:") {
        Principal::Group(name.into())
    } else if s == "everyone" {
        Principal::Everyone
    } else {
        Principal::User(s.into())
    }
}

fn parse_macos_perms(s: &str) -> Rights {
    let has = |k: &str| s.split(',').any(|p| p.trim() == k);
    Rights {
        read: has("read") || has("read_data"),
        write: has("write") || has("write_data"),
        execute: has("execute"),
        delete: has("delete"),
        append: has("append_data"),
        read_attr: has("read_attributes") || has("read_attr"),
        write_attr: has("write_attributes") || has("write_attr"),
        read_xattr: has("read_extattributes"),
        write_xattr: has("write_extattributes"),
        read_security: has("read_security"),
        write_security: has("write_security"),
        create_file: has("add_file"),
        create_dir: has("add_subdirectory"),
        list: has("list_directory"),
        ..Default::default()
    }
}

// ─── NFSv4 (nfs4_getfacl) ─────────────────────────────────────────────────────

fn read_nfs4(path: &Path, acl: &mut PathAcl) {
    acl.kind = AclKind::Nfs4Acl;
    let out = Command::new("nfs4_getfacl")
        .arg(path.to_str().unwrap_or(""))
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            acl.error = Some(e.to_string());
            return;
        }
    };
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    acl.raw_output = text.clone();
    parse_nfs4_output(&text, acl);
}

fn parse_nfs4_output(text: &str, acl: &mut PathAcl) {
    // Format: type:flags:principal:permissions
    // e.g.  A::alice@domain:rwatTnNcCy
    //        D:g:GROUP@:waxTC
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            continue;
        }
        let ace_type = parts[0];
        let flags = parts[1];
        let principal_str = parts[2];
        let perms = parts[3];

        let allow = ace_type == "A";
        let principal = match principal_str {
            "OWNER@" => Principal::Owner,
            "GROUP@" => Principal::OwningGroup,
            "EVERYONE@" => Principal::Everyone,
            other => {
                if flags.contains('g') {
                    Principal::Group(other.into())
                } else {
                    Principal::User(other.into())
                }
            }
        };

        let rights = parse_nfs4_perms(perms);
        let inherit_flags = parse_nfs4_inherit(flags);

        acl.aces.push(Ace {
            principal,
            allow,
            rights,
            inherit: inherit_flags,
            is_default: false,
        });
    }
}

fn parse_nfs4_perms(s: &str) -> Rights {
    let has = |c: char| s.contains(c);
    Rights {
        read: has('r'),
        write: has('w'),
        append: has('a'),
        execute: has('x'),
        delete: has('d'),
        read_attr: has('t'),
        write_attr: has('T'),
        read_xattr: has('n'),
        write_xattr: has('N'),
        read_security: has('c'),
        write_security: has('C'),
        synchronize: has('y'),
        ..Default::default()
    }
}

fn parse_nfs4_inherit(flags: &str) -> InheritFlags {
    InheritFlags {
        file_inherit: flags.contains('f'),
        dir_inherit: flags.contains('d'),
        inherit_only: flags.contains('i'),
        no_propagate: flags.contains('n'),
        inherited: flags.contains('I'),
        ..Default::default()
    }
}

// ─── Windows (icacls) ─────────────────────────────────────────────────────────

fn read_windows(path: &Path, acl: &mut PathAcl) {
    acl.kind = AclKind::WindowsDacl;
    let out = Command::new("icacls")
        .arg(path.to_str().unwrap_or(""))
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            acl.error = Some(e.to_string());
            return;
        }
    };
    // icacls uses OEM encoding on Windows; attempt UTF-8 with lossy fallback
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    acl.raw_output = text.clone();
    parse_icacls_output(&text, acl);
}

fn parse_icacls_output(text: &str, acl: &mut PathAcl) {
    // icacls output format:
    // C:\path\to\file DOMAIN\User:(flags)(rights)
    //                 NT AUTHORITY\SYSTEM:(OI)(CI)(F)
    //                 ...
    // Successfully processed 1 files; Failed processing 0 files
    let mut first = true;
    for line in text.lines() {
        if line.trim().is_empty()
            || line.contains("Successfully processed")
            || line.contains("Failed processing")
        {
            continue;
        }

        // Strip the path prefix from the first line
        let line = if first {
            first = false;
            let trimmed = line.trim();
            if let Some(ace_start) = trimmed.find(":(") {
                let before_ace = &trimmed[..ace_start];
                if let Some(last_space) = before_ace.rfind(' ') {
                    &trimmed[last_space + 1..]
                } else {
                    trimmed
                }
            } else {
                continue;
            }
        } else {
            line.trim()
        };

        parse_icacls_ace_line(line, acl);
    }
}

fn parse_icacls_ace_line(line: &str, acl: &mut PathAcl) {
    // Format: PRINCIPAL:(INHERIT_FLAGS)(RIGHTS)  or  PRINCIPAL:(RIGHTS)
    // There may be multiple paren groups.
    // Strategy: split on ':(' — everything before is the principal.
    let Some(colon_paren) = line.find(":(") else {
        return;
    };
    let principal_str = line[..colon_paren].trim();
    let rest = &line[colon_paren + 1..]; // starts with '('

    let mut inherit = InheritFlags::default();
    let mut rights = Rights::default();
    let mut allow = true;

    // Extract parenthesised tokens
    let mut s = rest;
    while let (Some(open), Some(close)) = (s.find('('), s.find(')')) {
        let token = &s[open + 1..close];
        s = &s[close + 1..];
        match token {
            "OI" => inherit.object_inherit = true,
            "CI" => inherit.container_inherit = true,
            "IO" => inherit.inherit_only = true,
            "NP" => inherit.no_propagate = true,
            "I" => inherit.inherited = true,
            // Rights tokens
            "F" => {
                rights.read = true;
                rights.write = true;
                rights.execute = true;
                rights.delete = true;
                rights.read_security = true;
                rights.write_security = true;
                rights.take_ownership = true;
            }
            "M" => {
                rights.read = true;
                rights.write = true;
                rights.execute = true;
                rights.delete = true;
            }
            "RX" => {
                rights.read = true;
                rights.execute = true;
            }
            "R" => {
                rights.read = true;
            }
            "W" => {
                rights.write = true;
            }
            "D" => {
                rights.delete = true;
            }
            "DENY" => {
                allow = false;
            }
            _ => {
                // Advanced permission list: e.g. (DE,RC,WDAC,...)
                for part in token.split(',') {
                    apply_icacls_advanced_right(part.trim(), &mut rights, &mut allow);
                }
            }
        }
    }

    let principal = parse_windows_principal(principal_str);
    acl.aces.push(Ace {
        principal,
        allow,
        rights,
        inherit,
        is_default: false,
    });
}

fn apply_icacls_advanced_right(part: &str, rights: &mut Rights, allow: &mut bool) {
    match part {
        "RD" => rights.read = true,
        "WD" => rights.write = true,
        "AD" => rights.append = true,
        "REA" => rights.read_xattr = true,
        "WEA" => rights.write_xattr = true,
        "X" => rights.execute = true,
        "DC" => rights.delete = true,
        "RA" => rights.read_attr = true,
        "WA" => rights.write_attr = true,
        "RC" => rights.read_security = true,
        "WDAC" => rights.write_security = true,
        "WO" => rights.take_ownership = true,
        "S" => rights.synchronize = true,
        "DE" => rights.delete = true,
        "DENY" => *allow = false,
        _ => {}
    }
}

fn parse_windows_principal(s: &str) -> Principal {
    if s.starts_with('S') && s.contains('-') {
        Principal::Sid(s.into())
    } else if s.eq_ignore_ascii_case("everyone") || s.eq_ignore_ascii_case("NT AUTHORITY\\Everyone")
    {
        Principal::Everyone
    } else if s.contains('\\') {
        let name = s.split('\\').last().unwrap_or(s);
        Principal::User(name.into())
    } else {
        Principal::User(s.into())
    }
}
