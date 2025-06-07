use crate::utils::is_valid_char;
use anyhow::Result;
use oxide_core::store::{HashPart, HASH_PART_LEN};
use sha2::Digest;
use std::collections::{HashMap, HashSet};
use std::io::SeekFrom;
use std::path::Path;
use tokio::io::AsyncSeekExt as _;
use tokio::io::AsyncWriteExt as _;
use tokio::{fs::OpenOptions, io::AsyncReadExt as _};

#[inline]
pub fn slice_to_array(buff: &[u8], i: usize) -> &[u8; HASH_PART_LEN] {
    if cfg!(debug_assertions) {
        buff[i..i + HASH_PART_LEN].try_into().unwrap()
    } else {
        unsafe { &*((&buff[i..i + HASH_PART_LEN]).as_ptr() as *const [u8; HASH_PART_LEN]) }
    }
}

#[inline]
pub fn zeroo_hash(buff: &mut [u8], i: usize) {
    buff[i..i + HASH_PART_LEN].fill(0);
}

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
        let ref_hash = slice_to_array(buff, i);
        if let Some(hash_part) = hashes.get(ref_hash) {
            occ.push((i, hash_part))
        }
        i += 1;
    }
    occ
}

pub async fn hash_and_rewrite<H, P>(
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

        for (match_pos, hash) in search(&mut tail, &mut hashes) {
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
        for (match_pos, hash) in search(&mut buff, &mut hashes) {
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

    hasher.update([255u8; 4]);
    for modulo in modulos {
        hasher.update([255u8; 4]);
        hasher.update(modulo.to_be_bytes());
    }
    let hash = hasher.finalize().to_vec();
    Ok(hash)
}
