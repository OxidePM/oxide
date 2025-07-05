pub mod config;

use crate::hash::Hash;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, fmt::Display, ops::Deref};

/// LENGTH of the base64 encoded hash without algo
pub const HASH_PART_LEN: usize = 64;
pub type HashPart<'a> = &'a [u8; HASH_PART_LEN];

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
/// A path inside of the store
// to be more general we could have used OsString
// but for our purposes it is useful to be able to
// serialize the data into utf-8 strings
pub struct StorePath(String);

impl StorePath {
    pub fn new(hash: &Hash, name: &str) -> Self {
        let mut h = hash.base64();
        h.truncate(HASH_PART_LEN);
        Self(format!("{h}-{name}"))
    }

    /// marked as unsafe because it does not guarantee to generate a real store path
    /// use wisely
    pub unsafe fn from_string(s: String) -> Self {
        Self(s)
    }

    /// marked as unsafe because it does not generate a real store path
    /// use wisely when generating fake store paths for instantiate
    pub unsafe fn empty() -> Self {
        Self(String::new())
    }

    pub fn name_part(&self) -> &str {
        &self.0[HASH_PART_LEN + 1..]
    }

    pub fn hash_part(&self) -> &str {
        &self.0[..HASH_PART_LEN]
    }

    pub fn hash_bytes(&self) -> HashPart<'_> {
        self.hash_part().as_bytes().try_into().unwrap()
    }

    pub fn rewrite_hash_part(&mut self, rewrite: &StorePath) {
        (unsafe { self.0[..HASH_PART_LEN].as_bytes_mut() }).copy_from_slice(rewrite.hash_bytes());
    }
}

impl Borrow<[u8; HASH_PART_LEN]> for StorePath {
    fn borrow(&'_ self) -> HashPart<'_> {
        self.hash_bytes()
    }
}

impl PartialEq<HashPart<'_>> for StorePath {
    fn eq(&self, other: &HashPart<'_>) -> bool {
        self.hash_bytes() == *other
    }
}

impl PartialEq<StorePath> for HashPart<'_> {
    fn eq(&self, other: &StorePath) -> bool {
        *self == other.hash_bytes()
    }
}

impl std::hash::Hash for StorePath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // only hash the hash part so that we do not have to pass HashPart eveywere
        // and consequently also lifetimes
        self.hash_bytes().hash(state);
    }
}

impl Display for StorePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for StorePath {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
