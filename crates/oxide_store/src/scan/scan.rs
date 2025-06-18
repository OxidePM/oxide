use super::search;
use anyhow::{bail, Result};
use oxide_core::store::{HashPart, StorePath, HASH_PART_LEN};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncReadExt as _,
};

pub async fn scan<P>(path: P, refs: &HashSet<StorePath>) -> Result<Vec<StorePath>>
where
    P: AsRef<Path>,
{
    let mut res = HashSet::new();
    let parts = refs
        .iter()
        .map(|r| r.to_hash_part())
        .collect::<HashSet<_>>();
    let part_to_path = refs
        .iter()
        .map(|r| (r.to_hash_part(), r))
        .collect::<HashMap<_, _>>();
    scan_root(path, &parts, &mut res).await?;
    Ok(res.into_iter().map(|r| part_to_path[r].clone()).collect())
}

async fn scan_root<'a, P>(
    path: P,
    refs: &'a HashSet<HashPart>,
    res: &mut HashSet<&'a HashPart>,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if path.is_dir() {
        Ok(scan_dir(path, refs, res).await?)
    } else if path.is_file() {
        Ok(scan_file(path, refs, res).await?)
    } else if path.is_symlink() {
        Ok(scan_symlink(path, refs, res).await?)
    } else {
        bail!("unknown file type")
    }
}

async fn scan_dir<'a, P>(
    path: P,
    refs: &'a HashSet<HashPart>,
    res: &mut HashSet<&'a HashPart>,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut entries = fs::read_dir(&path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        Box::pin(scan_root(path, refs, res)).await?
    }
    Ok(())
}

async fn scan_symlink<'a, P>(
    path: P,
    refs: &'a HashSet<HashPart>,
    res: &mut HashSet<&'a HashPart>,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let target = fs::read_link(path).await?;
    let buff = target.as_os_str().as_encoded_bytes();
    res.extend(search(buff, &refs).into_iter().map(|(_, h)| h));
    Ok(())
}

async fn scan_file<'a, P>(
    path: P,
    refs: &'a HashSet<HashPart>,
    res: &mut HashSet<&'a HashPart>,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut file = OpenOptions::new().read(true).open(path).await?;
    let mut buff = [0; 128 * 1024];

    let mut tail = Vec::new();
    loop {
        let n = file.read(&mut buff).await?;
        if n == 0 {
            break;
        }

        // TODO: test this code
        let m = HASH_PART_LEN.min(n);
        tail.extend_from_slice(&buff[..m]);

        res.extend(search(&tail, &refs).into_iter().map(|(_, h)| h));
        res.extend(search(&buff, &refs).into_iter().map(|(_, h)| h));

        tail.truncate(m);
        let rest = HASH_PART_LEN - m;
        if tail.len() > rest {
            tail.drain(..tail.len() - rest);
        }
        tail.extend_from_slice(&buff[n - m..n]);
    }
    Ok(())
}
