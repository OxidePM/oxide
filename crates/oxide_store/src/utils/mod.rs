use oxide_core::drv::DRV_EXT;
use std::path::{Path, PathBuf};

pub mod tempfile;

#[inline]
pub fn is_valid_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
}

pub fn is_valid_name(name: &str) -> bool {
    let min_len = 3 + if name.ends_with(DRV_EXT) {
        DRV_EXT.len()
    } else {
        0
    };
    name.len() >= min_len && name.chars().all(is_valid_char)
}

pub fn add_lock_ext<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let mut os_str = path.as_ref().as_os_str().to_os_string();
    os_str.push(".lock");
    os_str.into()
}
