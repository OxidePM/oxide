use crate::scan::search;
use anyhow::{bail, Result};
use oxide_core::hash::{Hash, HashAlgo};
use oxide_core::store::{HashPart, HASH_PART_LEN};
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashSet;
use std::io::SeekFrom;
use std::{
    collections::{BTreeMap, HashMap},
    os::unix::fs::PermissionsExt,
    path::Path,
};
use tokio::fs::{self, DirEntry};
use tokio::io::AsyncSeekExt as _;
use tokio::io::AsyncWriteExt as _;
use tokio::{fs::OpenOptions, io::AsyncReadExt as _};

pub async fn hash<P>(
    path: P,
    algo: HashAlgo,
    rewrites: &HashMap<HashPart, HashPart>,
    self_hash: Option<&HashPart>,
) -> Result<Hash>
where
    P: AsRef<Path>,
{
    macro_rules! hash_algos {
        ($($algo:pat, $hash:expr, $hasher:ty);*;) => {
            Ok(match algo {
            $(
                $algo => $hash(
                    must_hash_root::<$hasher, _>(path, rewrites, self_hash)
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

#[inline]
async fn must_hash_root<H, P>(
    path: P,
    rewrites: &HashMap<HashPart, HashPart>,
    self_hash: Option<&HashPart>,
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

pub async fn hash_symlink<H, P>(path: P) -> Result<Vec<u8>>
where
    H: Digest,
    P: AsRef<Path>,
{
    let target = fs::read_link(path).await?;
    Ok(H::digest(target.as_os_str().as_encoded_bytes()).to_vec())
}

#[inline]
pub fn zeroo_hash(buff: &mut [u8], i: usize) {
    buff[i..i + HASH_PART_LEN].fill(0);
}

pub async fn hash_file<H, P>(
    path: P,
    rewrites: &HashMap<HashPart, HashPart>,
    self_hash: Option<&HashPart>,
) -> Result<Vec<u8>>
where
    H: Digest,
    P: AsRef<Path>,
{
    let mut file = OpenOptions::new().read(true).write(true).open(path).await?;
    let mut buff = [0; 128 * 1024];

    let mut hasher = H::new();
    // TODO: make it so it does not always convert to HashSet
    let mut hashes = rewrites.keys().cloned().collect::<HashSet<_>>();
    if let Some(self_hash) = self_hash {
        hashes.insert(self_hash.clone());
    }

    let mut modulos = Vec::new();
    let mut tail = Vec::new();
    let mut offset = 0;
    loop {
        let n = file.read(&mut buff).await?;
        if n == 0 {
            break;
        }

        // TODO: test this code
        let m = HASH_PART_LEN.min(n);
        let tail_len = tail.len();
        tail.extend_from_slice(&buff[..m]);

        for (match_pos, hash) in search(&tail, &hashes) {
            let absolute_pos = offset + match_pos as u64 - tail_len as u64;
            if let Some(self_hash) = self_hash {
                if hash == self_hash {
                    zeroo_hash(&mut buff, match_pos);
                    modulos.push(absolute_pos);
                }
            }
            if let Some(rewrite) = rewrites.get(hash) {
                file.seek(SeekFrom::Start(absolute_pos)).await?;
                file.write_all(&**rewrite).await?;
            }
        }
        for (match_pos, hash) in search(&buff, &hashes) {
            let absolute_pos = offset + match_pos as u64;
            if let Some(self_hash) = self_hash {
                if hash == self_hash {
                    zeroo_hash(&mut buff, match_pos);
                    modulos.push(absolute_pos);
                }
            }
            if let Some(rewrite) = rewrites.get(hash) {
                file.seek(SeekFrom::Start(absolute_pos)).await?;
                file.write_all(&**rewrite).await?;
            }
        }

        tail.truncate(m);
        let rest = HASH_PART_LEN - m;
        if tail.len() > rest {
            tail.drain(..tail.len() - rest);
        }
        tail.extend_from_slice(&buff[n - m..n]);

        offset += n as u64;
        file.seek(SeekFrom::Start(offset)).await?;

        hasher.update(&buff);
    }

    hasher.update(u64::MAX.to_be_bytes());
    for modulo in modulos {
        hasher.update(u64::MAX.to_be_bytes());
        hasher.update(modulo.to_be_bytes());
    }
    let hash = hasher.finalize().to_vec();
    Ok(hash)
}
