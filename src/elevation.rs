use std::process::Command;

/// Re-launch elevated, passing the currently loaded path so the new instance
/// opens with the same state — no need to re-pick the file.
pub fn relaunch_elevated(current_path: Option<&str>) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let path_arg: Vec<String> = current_path
        .map(|p| vec![format!("--path={p}")])
        .unwrap_or_default();

    #[cfg(target_os = "windows")]
    {
        let path_flag = path_arg.first().cloned().unwrap_or_default();
        let cmd = if path_flag.is_empty() {
            format!("Start-Process -FilePath '{}' -Verb RunAs", exe.display())
        } else {
            format!(
                "Start-Process -FilePath '{}' -ArgumentList '{}' -Verb RunAs",
                exe.display(),
                path_flag
            )
        };
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &cmd])
            .spawn()
            .map_err(|e| e.to_string())?;
        std::process::exit(0);
    }

    #[cfg(target_os = "linux")]
    {
        let display = std::env::var("DISPLAY").unwrap_or_default();
        let xauth  = std::env::var("XAUTHORITY").unwrap_or_default();
        let home   = std::env::var("HOME").unwrap_or_default();

        let mut cmd = Command::new("pkexec");
        cmd.arg("env");
        if !display.is_empty() { cmd.arg(format!("DISPLAY={display}")); }
        if !xauth.is_empty()   { cmd.arg(format!("XAUTHORITY={xauth}")); }
        if !home.is_empty()    { cmd.arg(format!("HOME={home}")); }
        cmd.arg(&exe);
        for a in &path_arg { cmd.arg(a); }
        cmd.spawn().map_err(|e| e.to_string())?;
        std::process::exit(0);
    }

    #[cfg(target_os = "macos")]
    {
        // Build a shell command string that preserves TMPDIR and HOME
        // (required for the Mach window-server port and rfd file picker).
        let exe_str = exe.to_string_lossy();
        let path_flag = path_arg.first().cloned().unwrap_or_default();
        let tmpdir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
        let home   = std::env::var("HOME").unwrap_or_default();

        // We set TMPDIR and HOME explicitly so the elevated process can reach
        // the window server and resolve ~ paths correctly.
        let inner = if path_flag.is_empty() {
            format!(
                "export TMPDIR='{tmpdir}'; export HOME='{home}'; '{exe_str}'"
            )
        } else {
            format!(
                "export TMPDIR='{tmpdir}'; export HOME='{home}'; '{exe_str}' '{path_flag}'"
            )
        };
        let script = format!("do shell script \"bash -c '{inner}'\" with administrator privileges");
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| e.to_string())?;
        std::process::exit(0);
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err("Elevation not supported on this platform.".into())
    }
}
