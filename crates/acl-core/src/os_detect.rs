use std::path::Path;
use std::process::Command;

/// Which OS we are running on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Os {
    Windows,
    Macos,
    Linux,
    Other,
}

pub fn current_os() -> Os {
    if cfg!(target_os = "windows") {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::Macos
    } else if cfg!(target_os = "linux") {
        Os::Linux
    } else {
        Os::Other
    }
}

/// Which external tools are available on PATH.
#[derive(Debug, Clone, Default)]
pub struct ToolAvailability {
    pub getfacl: bool,
    pub setfacl: bool,
    pub nfs4_getfacl: bool,
    pub nfs4_setfacl: bool,
    pub icacls: bool,
    /// Whether the process is currently running as root/admin.
    pub is_elevated: bool,
}

fn has_tool(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or_else(|_| {
            // On Windows, try `where`
            Command::new("where")
                .arg(name)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
}

pub fn probe_tools() -> ToolAvailability {
    let is_elevated = check_elevated();
    match current_os() {
        Os::Windows => ToolAvailability {
            icacls: true,
            is_elevated,
            ..Default::default()
        },
        Os::Macos => ToolAvailability {
            nfs4_getfacl: has_tool("nfs4_getfacl"),
            nfs4_setfacl: has_tool("nfs4_setfacl"),
            is_elevated,
            ..Default::default()
        },
        Os::Linux => ToolAvailability {
            getfacl: has_tool("getfacl"),
            setfacl: has_tool("setfacl"),
            nfs4_getfacl: has_tool("nfs4_getfacl"),
            nfs4_setfacl: has_tool("nfs4_setfacl"),
            is_elevated,
            ..Default::default()
        },
        Os::Other => ToolAvailability {
            is_elevated,
            ..Default::default()
        },
    }
}

fn check_elevated() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(windows)]
    {
        // Check if current token has admin group
        is_admin_windows().unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

#[cfg(windows)]
fn is_admin_windows() -> Option<bool> {
    use windows::Win32::Foundation::*;
    use windows::Win32::Security::*;
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut elevated = BOOL(0);
        let mut token = HANDLE::default();
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY,
            &mut token,
        )
        .is_ok()
        {
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut len = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
            let _ = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                len,
                &mut len,
            );
            elevated = BOOL(elevation.TokenIsElevated as i32);
            let _ = CloseHandle(token);
        }
        Some(elevated.as_bool())
    }
}

/// Detect whether a path might be on an NFSv4 filesystem (heuristic: check
/// if `nfs4_getfacl` returns non-error on that path).
pub fn path_has_nfs4_acl(path: &Path, tools: &ToolAvailability) -> bool {
    if !tools.nfs4_getfacl {
        return false;
    }
    Command::new("nfs4_getfacl")
        .arg(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
