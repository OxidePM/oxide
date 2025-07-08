use crate::hash::utils::ChunkReader;
use anyhow::{Result, bail};
use oxide_core::store::{HASH_PART_LEN, HashPart, StorePath};
use std::collections::HashSet;
use std::path::Path;
use tokio::fs::{self, File};

use super::utils::is_valid_hash_char;

pub(crate) async fn scan_for_refs<P>(
    path: P,
    mut hashes: HashSet<StorePath>,
) -> Result<HashSet<StorePath>>
where
    P: AsRef<Path>,
{
    let mut res = HashSet::new();
    if scan_root(path, &mut hashes, &mut res).await?.is_none() {
        bail!("unknown file type");
    }
    Ok(res)
}

async fn scan_root<P>(
    path: P,
    hashes: &mut HashSet<StorePath>,
    res: &mut HashSet<StorePath>,
) -> Result<Option<()>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    Ok(if path.is_dir() {
        Some(scan_dir(path, hashes, res).await?)
    } else if path.is_file() {
        Some(scan_file(path, hashes, res).await?)
    } else if path.is_symlink() {
        Some(scan_symlink(path, hashes, res).await?)
    } else {
        None
    })
}

async fn scan_dir<P>(
    path: P,
    hashes: &mut HashSet<StorePath>,
    res: &mut HashSet<StorePath>,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut entries = fs::read_dir(&path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        _ = Box::pin(scan_root(path, hashes, res)).await?;
    }
    Ok(())
}

async fn scan_symlink<P>(
    path: P,
    hashes: &mut HashSet<StorePath>,
    res: &mut HashSet<StorePath>,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let target = fs::read_link(path).await?;
    let buff = target.as_os_str().as_encoded_bytes();
    search_refs(buff, hashes, res);
    Ok(())
}

async fn scan_file<P>(
    path: P,
    hashes: &mut HashSet<StorePath>,
    res: &mut HashSet<StorePath>,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let file = File::open(path).await?;

    let mut reader = ChunkReader::new(file);
    while let Some(mut chunk) = reader.next().await? {
        search_refs(chunk.chunk(), hashes, res);
    }
    Ok(())
}

/// search for refs
fn search_refs(buff: &[u8], hashes: &mut HashSet<StorePath>, res: &mut HashSet<StorePath>) {
    let mut i = 0;
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
        if let Some(hash) = hashes.take(ref_hash) {
            res.insert(hash);
        }
        i += 1;
    }
}
