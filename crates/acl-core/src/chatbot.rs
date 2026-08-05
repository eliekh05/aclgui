use crate::model::*;
use crate::os_detect::{Os, current_os};

/// A Q&A message in the help panel.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub from_user: bool,
    pub text: String,
}

/// Answer a plain-English question about a loaded PathAcl.
/// Returns a plain-English answer. No LLM required — purely rule-based.
pub fn answer(question: &str, context: Option<&PathAcl>) -> String {
    let q = question.to_lowercase();

    // Greetings
    if q.contains("hello") || q.contains("hi") || q == "?" {
        return "Hi! Ask me anything about the selected path's permissions. \
                E.g. \"why can't alice write here?\" or \"how do I add a group?\"".into();
    }

    // OS / tool questions
    if q.contains("what os") || q.contains("which os") || q.contains("platform") {
        let os = match current_os() {
            Os::Windows => "Windows — using icacls and the Win32 security API",
            Os::Macos   => "macOS — using ls -le and chmod ACL syntax",
            Os::Linux   => "Linux — using getfacl / setfacl (POSIX.1e) or nfs4_getfacl (NFSv4)",
            Os::Other   => "an unrecognised platform",
        };
        return format!("You are on {os}.");
    }

    if q.contains("acl kind") || q.contains("acl type") || q.contains("what kind") {
        if let Some(ctx) = context {
            let kind = match &ctx.kind {
                AclKind::PosixMode   => "POSIX mode bits only (no extended ACL).",
                AclKind::PosixAcl    => "POSIX.1e extended ACL (getfacl/setfacl).",
                AclKind::MacosAcl    => "macOS NFSv4-style ACL (chmod +a).",
                AclKind::WindowsDacl => "Windows NTFS DACL (icacls).",
                AclKind::Nfs4Acl     => "NFSv4 ACL (nfs4_getfacl/nfs4_setfacl).",
                AclKind::Unknown     => "Unknown — no path loaded yet.",
            };
            return format!("The selected path uses: {kind}");
        }
        return "No path loaded yet. Pick a file or directory first.".into();
    }

    // Why can't X do Y?
    if q.contains("why can't") || q.contains("why cannot") || q.contains("permission denied") || q.contains("access denied") {
        return answer_why_denied(context);
    }

    // Mask question
    if q.contains("mask") {
        return "On Linux POSIX ACLs, the mask entry caps the effective permissions of all \
                named user and group entries (but not owner or other). \
                For example: user:alice:rwx + mask::r-- → alice's effective rights are r-- only. \
                chmod g+w will change the mask, not the group entry. \
                Use setfacl -m m::rwx <path> to widen the mask.".into();
    }

    // Inheritance
    if q.contains("inherit") {
        return match current_os() {
            Os::Windows =>
                "On Windows, ACEs marked (OI) apply to files inside a folder, and (CI) apply to \
                 sub-directories. Inherited ACEs (shown with (I)) come from a parent and cannot \
                 be removed without first disabling inheritance (Disable Inheritance button).".into(),
            Os::Linux =>
                "On Linux, 'default ACLs' on a directory are inherited by new files and \
                 sub-directories created inside it. Use setfacl -d -m u:alice:rwx <dir> to set one.".into(),
            Os::Macos =>
                "On macOS, ACEs with file_inherit propagate to new files, and directory_inherit \
                 propagates to new sub-directories. Inherited entries are shown as 'inherited'.".into(),
            Os::Other => "Platform-specific inheritance rules apply.".into(),
        };
    }

    // How to add a user / group
    if q.contains("add") && (q.contains("user") || q.contains("group")) {
        return match current_os() {
            Os::Windows =>
                "Click \"Add ACE\", enter the user or group name (DOMAIN\\User or just User), \
                 choose Allow or Deny, pick the rights, then click \"Stage\". \
                 Apply the staged changes with the Apply button (requires admin).".into(),
            Os::Linux =>
                "Click \"Add ACE\", enter the user or group name, pick rwx rights, \
                 choose whether it is a default ACL, then Stage and Apply. \
                 This runs setfacl -m user:name:rwx <path>.".into(),
            Os::Macos =>
                "Click \"Add ACE\", enter the user or group name, choose allow/deny, \
                 pick rights and inheritance flags, then Stage and Apply. \
                 This runs chmod +a \"user:name allow read,write\" <path>.".into(),
            Os::Other => "Use your platform's native ACL tool.".into(),
        };
    }

    // Synchronize right (Windows gotcha)
    if q.contains("synchronize") || q.contains("sync") {
        return "On Windows, icacls silently adds the SYNCHRONIZE right when you deny certain \
                permissions (like Delete or Write). Denying SYNCHRONIZE locks out Explorer and \
                most apps entirely — even if your deny ACE only mentions D or W. \
                This GUI uses the Win32 API directly to avoid this icacls quirk, \
                and warns you if a DENY ACE would implicitly deny SYNCHRONIZE.".into();
    }

    // Setuid/setgid/sticky
    if q.contains("setuid") || q.contains("suid") || q.contains("setgid") || q.contains("sticky") {
        return "Setuid (s on owner execute bit): when set on an executable, it runs as the \
                file owner. Setgid (s on group execute bit): on directories, new files inherit \
                the directory's group. Sticky bit (t on other execute): on directories, only \
                the file owner can delete files inside (classic /tmp behaviour).".into();
    }

    // General help
    if q.contains("help") || q.contains("what can") || q.contains("what do") {
        return "I can answer questions like:\n\
                • \"Why can't alice write here?\"\n\
                • \"What is the mask?\"\n\
                • \"How do I add a group?\"\n\
                • \"What does inherit mean?\"\n\
                • \"What is Synchronize on Windows?\"\n\
                • \"What ACL type is this path using?\"\n\
                Pick a path first so I have context.".into();
    }

    // Default
    "I'm not sure about that one. Try: \"help\", \"what is the mask?\", \"why can't user write?\", or \"how do I add a group?\".".into()
}

fn answer_why_denied(context: Option<&PathAcl>) -> String {
    let Some(ctx) = context else {
        return "No path is loaded. Pick a file or directory first, then ask again.".into();
    };

    let mut reasons = Vec::<String>::new();

    // Check for explicit deny ACEs
    let deny_aces: Vec<_> = ctx.aces.iter().filter(|a| !a.allow).collect();
    if !deny_aces.is_empty() {
        let names: Vec<_> = deny_aces.iter().map(|a| a.principal.display()).collect();
        reasons.push(format!(
            "There are explicit DENY entries: {}. \
             DENY ACEs override ALLOW ACEs at the same or lower level.",
            names.join(", ")
        ));
    }

    // Check for mask narrowing (POSIX)
    if ctx.kind == AclKind::PosixAcl {
        let mask = ctx.aces.iter().find(|a| a.principal == Principal::Mask);
        if let Some(m) = mask {
            if !m.rights.write {
                reasons.push("The POSIX ACL mask does not include write. \
                              This caps named user/group entries to read-only effective rights. \
                              Fix: setfacl -m m::rwx <path>.".into());
            }
        }
    }

    // Check mode bits
    if let Some(mode) = &ctx.posix_mode {
        if !mode.other_read && !mode.other_write {
            reasons.push("The 'other' mode bits deny read+write to everyone not the owner or group.".into());
        }
    }

    // Windows synchronize trap
    if ctx.kind == AclKind::WindowsDacl {
        let has_deny_sync = ctx.aces.iter().any(|a| !a.allow && a.rights.synchronize);
        if has_deny_sync {
            reasons.push("A DENY entry includes the SYNCHRONIZE right, which locks out Explorer \
                          and most Win32 applications entirely.".into());
        }
    }

    if reasons.is_empty() {
        "No obvious blocking entry found in the loaded ACL. Possible causes not visible here:\n\
         • SELinux / AppArmor MAC policy (check audit.log or dmesg | grep denied)\n\
         • macOS SIP or TCC sandbox restriction\n\
         • The user is not in the expected group (run `id username` to check)\n\
         • File is on a read-only filesystem (check `mount` output)".into()
    } else {
        format!("Possible reason(s):\n{}", reasons.iter().map(|r| format!("• {r}")).collect::<Vec<_>>().join("\n"))
    }
}
