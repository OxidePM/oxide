use crate::system::System;
use std::{fs::Metadata, path::Path};

pub fn current_system() -> System {
    System::x86_64_linux
}

pub fn file_name<P>(p: P) -> String
where
    P: AsRef<Path>,
{
    p.as_ref()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
}

pub fn base_name(s: &str) -> &str {
    let end = s.len();
    if end == 0 {
        return s;
    }
    let end = if s.ends_with('/') { end - 1 } else { end };
    let start = s[..end].rfind('/').map_or(0, |start| start + 1);
    &s[start..end]
}

pub fn to_base_name(mut s: String) -> String {
    if s.ends_with('/') {
        s.pop();
    }
    let start = s.rfind('/').map_or(0, |start| start + 1);
    s.drain(0..start);
    s
}

pub const DIR_PERMISSION: u32 = 100_755;
pub const FILE_PERMISSION: u32 = 100_644;
pub const EXEC_FILE_PERMISSION: u32 = 100_644;
pub const SYMLINK_PERMISSION: u32 = 100_644;

#[inline]
pub fn file_type_to_permission(metadata: &Metadata) -> u32 {
    if metadata.is_dir() {
        DIR_PERMISSION
    } else if metadata.is_file() {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 != 0 {
            EXEC_FILE_PERMISSION
        } else {
            FILE_PERMISSION
        }
    } else if metadata.is_symlink() {
        SYMLINK_PERMISSION
    } else {
        0
    }
}

// TODO: change with proc macro
#[macro_export]
macro_rules! local_file {
    ($l:literal) => {
        $crate::expr::Expr::Path({
            let mut p = std::path::PathBuf::from(file!());
            p.pop();
            p.push($l);
            p.into()
        })
    };
}

pub use local_file;

// TODO: change with proc macro should panic at compile time
#[macro_export]
macro_rules! hash {
    ($l:literal) => {
        $crate::hash::Hash::try_from($l).unwrap()
    };
}

pub use hash;
