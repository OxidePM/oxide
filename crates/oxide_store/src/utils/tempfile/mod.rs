use rand::distr::Alphanumeric;
use rand::{Rng, rng};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::fs::File;
use tokio::io;

const NUM_RAND_CHARS: usize = 32;
const DOT: char = '.';

fn tmpname() -> OsString {
    (0..=NUM_RAND_CHARS)
        .map(|i| {
            if i == 0 {
                DOT
            } else {
                rng().sample(Alphanumeric) as char
            }
        })
        .collect::<String>()
        .into()
}

// TODO: tempfiles does not get deleted
// make sure you don't delete this file until we are done using it
// ensure the gc does not delete this
// what if the file exist?
// do we even want to handle that case
pub async fn tempfile_in<P>(p: P) -> io::Result<(File, PathBuf)>
where
    P: AsRef<Path>,
{
    let path = p.as_ref().join(tmpname());
    Ok((File::create(&path).await?, path))
}

// TODO: tempdir does not get deleted
pub async fn tempdir_in<P>(p: P) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
{
    let path = p.as_ref().join(tmpname());
    fs::create_dir(&path).await?;
    Ok(path)
}

pub fn is_temp<P>(p: P) -> bool
where
    P: AsRef<Path>,
{
    p.as_ref().file_name().is_some_and(|f| {
        f.as_encoded_bytes()
            .first()
            .is_some_and(|b| *b as char == DOT)
    })
}
