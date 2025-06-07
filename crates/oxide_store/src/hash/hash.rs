use super::scan::hash_and_rewrite;
use anyhow::{bail, Result};
use oxide_core::hash::{Hash, HashAlgo};
use oxide_core::store::HashPart;
use sha2::{Digest, Sha256, Sha512};
use std::{
    collections::{BTreeMap, HashMap},
    os::unix::fs::PermissionsExt,
    path::Path,
};
use tokio::fs::{self, DirEntry};

pub async fn hash<P>(
    path: P,
    algo: HashAlgo,
    rewrites: &HashMap<HashPart, HashPart>,
    self_hash: Option<&HashPart>,
) -> Result<Hash>
where
    P: AsRef<Path>,
{
    match algo {
        HashAlgo::Sha256 => Ok(Hash::Sha256(
            if let Some(hash) = hash_root::<Sha256, _>(path, rewrites, self_hash).await? {
                hash.try_into().unwrap()
            } else {
                bail!("unknown file type");
            },
        )),
        HashAlgo::Sha512 => Ok(Hash::Sha512(
            if let Some(hash) = hash_root::<Sha512, _>(path, rewrites, self_hash).await? {
                hash.try_into().unwrap()
            } else {
                bail!("unknown file type");
            },
        )),
        _ => bail!("unimplemented hash algo"),
    }
}

async fn hash_root<H, P>(
    path: P,
    rewrites: &HashMap<HashPart, HashPart>,
    self_hash: Option<&HashPart>,
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
        Some(hash_symlink::<H, _>(path).await?)
    } else {
        None
    })
}

#[inline]
pub async fn hash_file<H, P>(
    path: P,
    rewrites: &HashMap<HashPart, HashPart>,
    self_hash: Option<&HashPart>,
) -> Result<Vec<u8>>
where
    H: Digest,
    P: AsRef<Path>,
{
    let hash = hash_and_rewrite::<H, _>(&path, rewrites, self_hash).await?;
    Ok(hash)
}

pub async fn hash_symlink<H, P>(path: P) -> Result<Vec<u8>>
where
    H: Digest,
    P: AsRef<Path>,
{
    let target = fs::read_link(path).await?;
    Ok(Sha512::digest(target.as_os_str().as_encoded_bytes()).to_vec())
}

pub const DIR_PERMISSION: u64 = 100755;
pub const FILE_PERMISSION: u64 = 100644;
pub const EXEC_FILE_PERMISSION: u64 = 100644;
pub const SYMLINK_PERMISSION: u64 = 100644;

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

pub async fn hash_dir<H, P>(
    path: P,
    rewrites: &HashMap<HashPart, HashPart>,
    self_hash: Option<&HashPart>,
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
