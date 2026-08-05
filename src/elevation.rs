use std::process::Command;

/// On platforms where the running process is not elevated, we re-launch
/// ourselves via the appropriate privilege-elevation mechanism.
/// This is called from the UI when the user clicks "Apply with Elevation".
pub fn relaunch_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        // ShellExecuteEx with "runas" verb triggers UAC
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Start-Process -FilePath '{}' -Verb RunAs", exe.display()),
            ])
            .spawn()
            .map_err(|e| e.to_string())?;
        // Close the current non-elevated instance
        std::process::exit(0);
    }

    #[cfg(target_os = "linux")]
    {
        let display = std::env::var("DISPLAY").unwrap_or_default();
        let xauth = std::env::var("XAUTHORITY").unwrap_or_default();
        let home = std::env::var("HOME").unwrap_or_default();

        let mut cmd = Command::new("pkexec");
        cmd.arg("env");
        if !display.is_empty() {
            cmd.arg(format!("DISPLAY={display}"));
        }
        if !xauth.is_empty() {
            cmd.arg(format!("XAUTHORITY={xauth}"));
        }
        if !home.is_empty() {
            cmd.arg(format!("HOME={home}"));
        }
        cmd.arg(exe);
        cmd.spawn().map_err(|e| e.to_string())?;
        std::process::exit(0);
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: osascript can show an admin password dialog
        let script = format!(
            r#"do shell script "{exe}" with administrator privileges"#,
            exe = exe.display()
        );
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
