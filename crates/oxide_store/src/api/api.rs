use super::Opt;
use crate::utils::tempfile::tempfile_in;
use anyhow::Result;
use oxide_core::store::{globals::Config, StorePath};
use std::cell::LazyCell;
use std::path::Path;
use tokio::io::{self, AsyncBufRead, BufReader, BufWriter};

pub const CONFIG: LazyCell<Config> = LazyCell::new(|| Config::new());

#[allow(async_fn_in_trait)]
pub trait Store {
    async fn add_to_store<P>(&self, path: P, opt: Opt) -> Result<StorePath>
    where
        P: AsRef<Path>;

    async fn add_to_store_buff<R>(&self, mut buff: BufReader<R>, opt: Opt) -> Result<StorePath>
    where
        R: AsyncBufRead + Unpin,
    {
        let (file, path) = tempfile_in(&CONFIG.store_dir).await?;
        let mut dst = BufWriter::new(file);
        io::copy(&mut buff, &mut dst).await?;
        self.add_to_store(
            path,
            Opt {
                algo: opt.algo,
                refs: opt.refs,
                eq_refs: opt.eq_refs,
                name: opt.name,
                rewrites: opt.rewrites,
                self_hash: opt.self_hash,
            },
        )
        .await
    }

    fn real_store_path(&self, path: &StorePath) -> String;
}
