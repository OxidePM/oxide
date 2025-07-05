use super::utils::is_valid_hash_char;
use crate::hash::{rewrite_hash, search_rewrites, utils::ChunkReader};
use anyhow::{bail, Result};
use oxide_core::store::{HashPart, StorePath, HASH_PART_LEN};
use std::{collections::HashMap, io::SeekFrom, path::Path};
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncSeekExt, AsyncWriteExt},
};

pub fn rewrite_str(s: &mut str, rewrites: &HashMap<StorePath, StorePath>) {
    let bytes = unsafe { s.as_bytes_mut() };
    let occ = search_rewrites(bytes, rewrites, None);
    for (i, rewrite) in occ {
        rewrite_hash(bytes, i, rewrite);
    }
}

pub fn rewrite_store_path(path: &mut StorePath, rewrites: &HashMap<StorePath, StorePath>) {
    if let Some(rewrite) = rewrites.get(path.hash_bytes()) {
        path.rewrite_hash_part(rewrite);
    }
}

pub async fn rewrite_self_hash<P>(path: P, self_hash: &StorePath, rewrite: &StorePath) -> Result<()>
where
    P: AsRef<Path>,
{
    if rewrite_root(path, self_hash, rewrite).await?.is_none() {
        bail!("unknown file type")
    } else {
        Ok(())
    }
}

async fn rewrite_root<P>(path: P, self_hash: &StorePath, rewrite: &StorePath) -> Result<Option<()>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    Ok(if path.is_dir() {
        Some(rewrite_dir(path, self_hash, rewrite).await?)
    } else if path.is_file() {
        Some(rewrite_file(path, self_hash, rewrite).await?)
    } else if path.is_symlink() {
        Some(rewrite_symlink(path, self_hash, rewrite).await?)
    } else {
        None
    })
}

async fn rewrite_dir<P>(path: P, self_hash: &StorePath, rewrite: &StorePath) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut entries = fs::read_dir(&path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        _ = Box::pin(rewrite_root(path, self_hash, rewrite)).await?;
    }
    Ok(())
}

// TODO: rewrite symlink
async fn rewrite_symlink<P>(path: P, self_hash: &StorePath, _rewrite: &StorePath) -> Result<()>
where
    P: AsRef<Path>,
{
    let target = fs::read_link(path).await?;
    let buff = target.as_os_str().as_encoded_bytes();
    #[allow(clippy::never_loop)]
    for _i in search_self_hash(buff, self_hash) {
        unimplemented!()
    }
    Ok(())
}

async fn rewrite_file<P>(path: P, self_hash: &StorePath, rewrite: &StorePath) -> Result<()>
where
    P: AsRef<Path>,
{
    let reader = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .await?;
    let mut writer = reader.try_clone().await?;

    let mut reader = ChunkReader::new(reader);
    while let Some(mut chunk) = reader.next().await? {
        for i in search_self_hash(chunk.chunk(), self_hash) {
            let absolute_pos = chunk.chunk_offset() + i as u64;
            writer.seek(SeekFrom::Start(absolute_pos)).await?;
            writer.write_all(rewrite.hash_bytes()).await?;
        }
    }

    Ok(())
}

fn search_self_hash(buff: &[u8], self_hash: &StorePath) -> Vec<usize> {
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
        if &ref_hash == self_hash {
            occ.push(i)
        }
        i += 1;
    }
    occ
}
