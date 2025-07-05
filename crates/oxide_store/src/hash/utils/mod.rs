mod chunk;

pub use chunk::*;

use oxide_core::{hash::Hash, store::StorePath};
use sha2::{Digest, Sha512};

pub fn make_path(h: &Hash, name: &str) -> StorePath {
    let mut hasher = Sha512::new();
    hasher.update(format!("{}:{}", h, name));
    let hash = hasher.finalize();
    let hash = Hash::Sha512(Box::new(hash.into()));
    StorePath::new(hash, name)
}

pub fn random_hash() -> Hash {
    let hash: [u8; 64] = rand::random();
    Hash::Sha512(Box::new(hash))
}

pub fn random_path(name: &str) -> StorePath {
    StorePath::new(random_hash(), name)
}

#[inline]
pub fn is_valid_hash_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}
