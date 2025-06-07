use crate::system::System;
use std::path::Path;

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
    let end = if s.ends_with("/") { end - 1 } else { end };
    let start = s[..end].rfind("/").unwrap_or(0);
    &s[start..end]
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
