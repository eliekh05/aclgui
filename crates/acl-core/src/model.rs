use serde::{Deserialize, Serialize};

/// Which ACL subsystem a path is managed by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AclKind {
    /// Standard Unix mode bits only (rwxr-xr-x), no extended ACL.
    PosixMode,
    /// POSIX.1e draft ACL (Linux ext4/xfs/btrfs, `getfacl`/`setfacl`).
    PosixAcl,
    /// macOS NFSv4-style ACL via `ls -le` / `chmod +a`.
    MacosAcl,
    /// Windows NTFS DACL (icacls / Win32 security API).
    WindowsDacl,
    /// NFSv4 ACL on Linux/macOS NFS mounts (`nfs4_getfacl`/`nfs4_setfacl`).
    Nfs4Acl,
    /// Could not determine; display raw output only.
    Unknown,
}

/// Unified principal: who an ACE applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Principal {
    User(String),
    Group(String),
    /// Windows SID that couldn't be resolved to a name.
    Sid(String),
    /// POSIX special entries.
    Owner,
    OwningGroup,
    Other,
    Everyone,
    Mask,
}

impl Principal {
    pub fn display(&self) -> String {
        match self {
            Self::User(n) => format!("user:{n}"),
            Self::Group(n) => format!("group:{n}"),
            Self::Sid(s) => format!("SID:{s}"),
            Self::Owner => "owner".into(),
            Self::OwningGroup => "owning-group".into(),
            Self::Other => "other".into(),
            Self::Everyone => "everyone".into(),
            Self::Mask => "mask".into(),
        }
    }
}

/// Individual permission rights — superset across all OSes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Rights {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub delete: bool,
    pub list: bool,
    pub create_file: bool,
    pub create_dir: bool,
    pub read_attr: bool,
    pub write_attr: bool,
    pub read_xattr: bool,
    pub write_xattr: bool,
    pub read_security: bool,
    pub write_security: bool,
    pub take_ownership: bool,
    pub append: bool,
    pub synchronize: bool,
    /// Raw permission string when we can't fully parse.
    pub raw: Option<String>,
}

impl Rights {
    pub fn summary(&self) -> String {
        let mut out = String::new();
        if self.read || self.list { out.push('r'); } else { out.push('-'); }
        if self.write || self.create_file || self.create_dir { out.push('w'); } else { out.push('-'); }
        if self.execute { out.push('x'); } else { out.push('-'); }
        out
    }
}

/// Inheritance flags (Windows / NFSv4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InheritFlags {
    pub object_inherit: bool,
    pub container_inherit: bool,
    pub inherit_only: bool,
    pub no_propagate: bool,
    pub inherited: bool,
    pub file_inherit: bool,
    pub dir_inherit: bool,
}

/// A single access control entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ace {
    pub principal: Principal,
    pub allow: bool,
    pub rights: Rights,
    pub inherit: InheritFlags,
    pub is_default: bool,
}

/// POSIX mode bits.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PosixMode {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_execute: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_execute: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_execute: bool,
    pub setuid: bool,
    pub setgid: bool,
    pub sticky: bool,
}

impl PosixMode {
    pub fn from_octal(octal: u32) -> Self {
        Self {
            setuid:          octal & 0o4000 != 0,
            setgid:          octal & 0o2000 != 0,
            sticky:          octal & 0o1000 != 0,
            owner_read:      octal & 0o0400 != 0,
            owner_write:     octal & 0o0200 != 0,
            owner_execute:   octal & 0o0100 != 0,
            group_read:      octal & 0o0040 != 0,
            group_write:     octal & 0o0020 != 0,
            group_execute:   octal & 0o0010 != 0,
            other_read:      octal & 0o0004 != 0,
            other_write:     octal & 0o0002 != 0,
            other_execute:   octal & 0o0001 != 0,
        }
    }

    pub fn to_octal(&self) -> u32 {
        let mut o = 0u32;
        if self.setuid        { o |= 0o4000; }
        if self.setgid        { o |= 0o2000; }
        if self.sticky        { o |= 0o1000; }
        if self.owner_read    { o |= 0o0400; }
        if self.owner_write   { o |= 0o0200; }
        if self.owner_execute { o |= 0o0100; }
        if self.group_read    { o |= 0o0040; }
        if self.group_write   { o |= 0o0020; }
        if self.group_execute { o |= 0o0010; }
        if self.other_read    { o |= 0o0004; }
        if self.other_write   { o |= 0o0002; }
        if self.other_execute { o |= 0o0001; }
        o
    }

    pub fn symbolic(&self) -> String {
        let bit = |b: bool, c: char| if b { c } else { '-' };
        format!(
            "{}{}{}{}{}{}{}{}{}",
            bit(self.owner_read,    'r'), bit(self.owner_write,    'w'), bit(self.owner_execute,   'x'),
            bit(self.group_read,    'r'), bit(self.group_write,    'w'), bit(self.group_execute,   'x'),
            bit(self.other_read,    'r'), bit(self.other_write,    'w'), bit(self.other_execute,   'x'),
        )
    }
}

/// The full permission state for a path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathAcl {
    pub path: String,
    pub kind: AclKind,
    pub is_dir: bool,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub posix_mode: Option<PosixMode>,
    /// Extended ACL entries (empty if none).
    pub aces: Vec<Ace>,
    /// Default ACL entries (directories, POSIX/NFSv4).
    pub default_aces: Vec<Ace>,
    /// Raw output from the OS tool for display/debug.
    pub raw_output: String,
    /// Error encountered, if any.
    pub error: Option<String>,
}

impl PathAcl {
    pub fn empty(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: AclKind::Unknown,
            is_dir: false,
            owner: None,
            group: None,
            posix_mode: None,
            aces: vec![],
            default_aces: vec![],
            raw_output: String::new(),
            error: None,
        }
    }
}

/// A staged change to be applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    SetMode { octal: u32 },
    SetOwner { user: String },
    SetGroup { group: String },
    AddAce { ace: Ace, default: bool },
    RemoveAce { index: usize, default: bool },
    ModifyAce { index: usize, ace: Ace, default: bool },
    DisableInheritance { copy_existing: bool },
    EnableInheritance,
    RemoveAllAces,
}

/// A staged set of changes for one path.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeSet {
    pub path: String,
    pub changes: Vec<Change>,
}
