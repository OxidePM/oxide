use crate::hash::utils::ChunkReader;
use anyhow::{bail, Result};
use oxide_core::hash::{Hash, HashAlgo};
use oxide_core::store::{HashPart, StorePath, HASH_PART_LEN};
use sha2::{Digest, Sha256, Sha512};
use std::io::SeekFrom;
use std::{
    collections::{BTreeMap, HashMap},
    os::unix::fs::PermissionsExt,
    path::Path,
};
use tokio::fs::OpenOptions;
use tokio::fs::{self, DirEntry};
use tokio::io::AsyncSeekExt as _;
use tokio::io::AsyncWriteExt as _;

use super::utils::is_valid_hash_char;

/// To not have to read the file system twice
/// this function hashes and applies rewrites at the same time
pub(crate) async fn hash_mod_rewrites<P>(
    path: P,
    algo: HashAlgo,
    rewrites: &HashMap<StorePath, StorePath>,
    self_hash: Option<&StorePath>,
) -> Result<Hash>
where
    P: AsRef<Path>,
{
    #[inline]
    async fn must_hash_root<H, P>(
        path: P,
        rewrites: &HashMap<StorePath, StorePath>,
        self_hash: Option<&StorePath>,
    ) -> Result<Vec<u8>>
    where
        H: Digest,
        P: AsRef<Path>,
    {
        if let Some(hash) = hash_root::<H, P>(path, rewrites, self_hash).await? {
            Ok(hash)
        } else {
            bail!("unknown file type")
        }
    }

    macro_rules! hash_algos {
        ($($algo:pat, $hash:expr, $hasher:ty);*;) => {
            Ok(match algo {
            $(
                $algo => $hash(
                    must_hash_root::<$hasher, _>(path, &rewrites, self_hash)
                        .await?
                        .try_into()
                        .unwrap(),
                ),
            )*
                _ => bail!("unimplemented hash algo"),
            })
        };
    }
    hash_algos!(
        HashAlgo::Sha256, Hash::Sha256, Sha256;
        HashAlgo::Sha512, Hash::Sha512, Sha512;
    )
}

async fn hash_root<H, P>(
    path: P,
    rewrites: &HashMap<StorePath, StorePath>,
    self_hash: Option<&StorePath>,
) -> Result<Option<Vec<u8>>>
where
    H: Digest,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    Ok(if path.is_dir() {
        Some(hash_dir::<H, _>(path, rewrites, self_hash).await?)
    } else if path.is_file() {
        Some(hash_file::<H, _>(path, rewrites, self_hash).await?)
    } else if path.is_symlink() {
        Some(hash_symlink::<H, _>(path, rewrites, self_hash).await?)
    } else {
        None
    })
}

pub const DIR_PERMISSION: u64 = 100_755;
pub const FILE_PERMISSION: u64 = 100_644;
pub const EXEC_FILE_PERMISSION: u64 = 100_644;
pub const SYMLINK_PERMISSION: u64 = 100_644;

pub async fn file_type_to_permission(entry: DirEntry) -> Result<u64> {
    let metadata = entry.metadata().await?;
    Ok(if metadata.is_dir() {
        DIR_PERMISSION
    } else if metadata.is_file() {
        if metadata.permissions().mode() == libc::S_IEXEC {
            EXEC_FILE_PERMISSION
        } else {
            FILE_PERMISSION
        }
    } else if metadata.is_symlink() {
        SYMLINK_PERMISSION
    } else {
        0
    })
}

async fn hash_dir<H, P>(
    path: P,
    rewrites: &HashMap<StorePath, StorePath>,
    self_hash: Option<&StorePath>,
) -> Result<Vec<u8>>
where
    H: Digest,
    P: AsRef<Path>,
{
    let mut entries = fs::read_dir(&path).await?;

    let mut sorted_entries = BTreeMap::new();
    while let Some(entry) = entries.next_entry().await? {
        sorted_entries.insert(entry.file_name(), entry);
    }

    let mut hasher = H::new();
    for (_, entry) in sorted_entries {
        let file_name = entry.file_name();
        let path = entry.path();

        let Some(hash) = Box::pin(hash_root::<H, _>(path, rewrites, self_hash)).await? else {
            continue;
        };
        let perm = file_type_to_permission(entry).await?;
        hasher.update(perm.to_be_bytes());
        hasher.update((file_name.len() as u64).to_be_bytes());
        hasher.update(file_name.as_encoded_bytes());
        hasher.update((hash.len() as u64).to_be_bytes());
        hasher.update(hash);
    }

    let hash = hasher.finalize();
    Ok(hash.to_vec())
}

// TODO: hash symlink
async fn hash_symlink<H, P>(
    path: P,
    rewrites: &HashMap<StorePath, StorePath>,
    self_hash: Option<&StorePath>,
) -> Result<Vec<u8>>
where
    H: Digest,
    P: AsRef<Path>,
{
    let target = fs::read_link(path).await?;
    let buff = target.as_os_str().as_encoded_bytes();
    #[allow(clippy::never_loop)]
    for (_i, _hash) in search_rewrites(buff, rewrites, self_hash) {
        unimplemented!()
    }
    Ok(H::digest(buff).to_vec())
}

#[inline]
pub(crate) fn zeroo_hash(buff: &mut [u8], i: usize) {
    buff[i..i + HASH_PART_LEN].fill(0);
}

#[inline]
pub(crate) fn rewrite_hash(buff: &mut [u8], i: usize, rewrite: &StorePath) {
    buff[i..i + HASH_PART_LEN].copy_from_slice(rewrite.hash_bytes());
}

async fn hash_file<H, P>(
    path: P,
    rewrites: &HashMap<StorePath, StorePath>,
    self_hash: Option<&StorePath>,
) -> Result<Vec<u8>>
where
    H: Digest,
    P: AsRef<Path>,
{
    let reader = OpenOptions::new().read(true).write(true).open(path).await?;
    let mut writer = reader.try_clone().await?;

    let mut hasher = H::new();
    let mut modulos = Vec::new();

    let mut reader = ChunkReader::new(reader);
    let mut remaining = Vec::new();
    while let Some(mut chunk) = reader.next().await? {
        for (i, hash) in search_rewrites(chunk.chunk(), rewrites, self_hash) {
            let absolute_pos = chunk.chunk_offset() + i as u64;
            if let Some(rewrite) = rewrites.get(hash) {
                rewrite_hash(chunk.chunk(), i, rewrite);
                writer.seek(SeekFrom::Start(absolute_pos)).await?;
                writer.write_all(rewrite.hash_bytes()).await?;
            } else if let Some(self_hash) = self_hash {
                if self_hash == hash {
                    zeroo_hash(chunk.chunk(), i);
                    modulos.push(absolute_pos);
                }
            }
        }
        let (lhs, rhs) = chunk.split_at_overlap();
        hasher.update(lhs);
        remaining = rhs.to_vec();
    }
    hasher.update(remaining);

    hasher.update(u64::MAX.to_be_bytes());
    for modulo in modulos {
        hasher.update(u64::MAX.to_be_bytes());
        hasher.update(modulo.to_be_bytes());
    }
    let hash = hasher.finalize().to_vec();
    Ok(hash)
}

/// search for `rewrites` and `self_hash`
pub(super) fn search_rewrites<'a>(
    buff: &[u8],
    rewrites: &'a HashMap<StorePath, StorePath>,
    self_hash: Option<&'a StorePath>,
) -> Vec<(usize, &'a StorePath)> {
    let mut i = 0;
    let mut occ = Vec::new();
    'outer: while i + HASH_PART_LEN <= buff.len() {
        let mut j = i + HASH_PART_LEN - 1;
        loop {
            if !is_valid_hash_char(buff[j] as char) {
                i = j + 1;
                continue 'outer;
            }
            if j <= i {
                break;
            }
            j -= 1;
        }
        let ref_hash: HashPart<'_> = &buff[i..i + HASH_PART_LEN].try_into().unwrap();
        if let Some(rewrite) = rewrites.get(ref_hash) {
            occ.push((i, rewrite));
        } else if let Some(self_hash) = self_hash {
            if ref_hash == self_hash.hash_bytes() {
                occ.push((i, self_hash));
            }
        }
        i += 1;
    }
    occ
}
