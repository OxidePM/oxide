mod scan;

pub use scan::*;

use crate::utils::is_valid_char;
use oxide_core::store::{HashPart, HASH_PART_LEN};
use std::collections::HashSet;

pub fn search<'a>(buff: &[u8], hashes: &'a HashSet<HashPart>) -> Vec<(usize, &'a HashPart)> {
    let mut i = 0;
    let mut occ = Vec::new();
    'outer: while i + HASH_PART_LEN <= buff.len() {
        let mut j = i + HASH_PART_LEN - 1;
        while j >= i {
            if !is_valid_char(buff[j] as char) {
                i = j + 1;
                continue 'outer;
            }
            j -= 1;
        }
        let ref_hash: &[u8; HASH_PART_LEN] = &buff[i..i + HASH_PART_LEN].try_into().unwrap();
        if let Some(hash_part) = hashes.get(ref_hash) {
            occ.push((i, hash_part))
        }
        i += 1;
    }
    occ
}
