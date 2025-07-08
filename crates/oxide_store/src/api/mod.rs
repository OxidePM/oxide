mod opt;
pub use opt::*;

use crate::{types::Realisation, utils::tempfile::tempfile_in};
use anyhow::Result;
use oxide_core::{
    drv::StoreDrv,
    store::{config::Config, StorePath},
    types::{EqClass, Out},
};
use std::cell::LazyCell;
use std::path::Path;
use tokio::{
    fs,
    io::{self, AsyncBufRead, BufReader},
};

// TODO: maybe do not use a global variable
pub const CONFIG: LazyCell<Config> = LazyCell::new(Config::new);

#[allow(async_fn_in_trait)]
pub trait Store {
    async fn add_to_store<P>(&self, path: P, opt: Opt) -> Result<StorePath>
    where
        P: AsRef<Path>;

    async fn add_to_store_buff<R>(&self, mut buff: BufReader<R>, opt: Opt) -> Result<StorePath>
    where
        R: AsyncBufRead + Unpin,
    {
        let (mut file, path) = tempfile_in(&CONFIG.store_dir).await?;
        io::copy(&mut buff, &mut file).await?;
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

    fn store_dir() -> String {
        CONFIG.store_dir.to_string()
    }

    fn store_path(path: &StorePath) -> String {
        format!("{}/{}", CONFIG.store_dir, path)
    }

    async fn read_drv(&self, p: &StorePath) -> Result<StoreDrv> {
        let path = Self::store_path(p);
        let buff = fs::read(&path).await?;
        let drv_str = String::from_utf8(buff).unwrap();
        Ok(toml::from_str(&drv_str)?)
    }

    async fn trusted_paths(&self, eq_class: &EqClass, out: &Out) -> Result<Vec<StorePath>>;

    async fn realisation_refs(&self, realisation: &Realisation) -> Result<Vec<Realisation>>;
}
