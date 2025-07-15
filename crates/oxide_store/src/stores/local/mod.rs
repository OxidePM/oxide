mod queries;

use crate::api::{CONFIG, Opt, Store};
use crate::hash::utils::make_path;
use crate::hash::{hash_mod_rewrites, rewrite_self_hash, rewrite_store_path};
use crate::os::lock::{LockMode, PathLock};
use crate::types::{Realisation, StoreObj};
use crate::utils::{add_lock_ext, is_valid_name};
use anyhow::{Result, bail};
use log::info;
use oxide_core::store::StorePath;
use oxide_core::types::{EqClass, Out};
use oxide_core::utils::file_type_to_permission;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use std::cell::LazyCell;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;

pub struct LocalStoreConfig {
    pub db_dir: String,
    pub db_path: String,
    pub migrations_dir: String,
}

impl LocalStoreConfig {
    pub fn new() -> Self {
        let db_dir = format!("{}/db", CONFIG.state_dir);
        let db_path = format!("{db_dir}/sqlite.db");
        let migrations_dir = format!("{db_dir}/migrations");
        Self {
            db_dir,
            db_path,
            migrations_dir,
        }
    }
}

impl Default for LocalStoreConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub const LOCAL_STORE_CONFIG: LazyCell<LocalStoreConfig> = LazyCell::new(LocalStoreConfig::new);

pub struct LocalStore {
    db: SqlitePool,
}

impl LocalStore {
    pub async fn new() -> Result<Self> {
        let db = SqlitePool::connect(&format!("sqlite://{}", LOCAL_STORE_CONFIG.db_path)).await?;
        let m = Migrator::new(PathBuf::from(&LOCAL_STORE_CONFIG.migrations_dir)).await?;
        m.run(&db).await?;
        Ok(Self { db })
    }
}

impl Store for LocalStore {
    async fn add_to_store<P>(&self, p: P, mut opt: Opt) -> Result<StorePath>
    where
        P: AsRef<Path>,
    {
        if !is_valid_name(&opt.name) {
            bail!("invalid name: {}", opt.name);
        }
        // TODO: add fix flag
        let fix = false;

        let hash = hash_mod_rewrites(&p, opt.algo, &opt.rewrites, opt.self_hash.as_ref()).await?;
        let path = make_path(&hash, &opt.name);
        if fix || !self.valid(&path).await? {
            info!("add to store: {path}");
            let full_path = Self::store_path(&path);
            let lock_file = add_lock_ext(&full_path);
            let lock = PathLock::lock(lock_file, LockMode::Write)?;
            if fix || !self.valid(&path).await? {
                self.copy_path(&p, &full_path).await?;
                if let Some(self_hash) = opt.self_hash {
                    rewrite_self_hash(&full_path, &self_hash, &path).await?;
                    opt.rewrites.insert(self_hash, path.clone());
                }
                let mut refs = Vec::new();
                for mut r in opt.refs {
                    rewrite_store_path(&mut r, &opt.rewrites);
                    refs.push(r);
                }
                self.register_store_obj(
                    StoreObj {
                        path: path.clone(),
                        hash,
                    },
                    refs,
                )
                .await?;
            }
            lock.unlock();
        }
        if let Some(mut eq_refs) = opt.eq_refs {
            for eq_ref in &mut eq_refs.refs {
                rewrite_store_path(&mut eq_ref.path, &opt.rewrites);
            }
            self.register_realisation(
                Realisation {
                    eq_class: eq_refs.eq_class,
                    out: eq_refs.out,
                    path: path.clone(),
                },
                eq_refs.refs,
            )
            .await?;
        }

        Ok(path)
    }

    // TODO: for now every path is trusted
    // this is not secure and it must be changed
    // maybe add a set of trusted distributors in config
    async fn trusted_paths(&self, eq_class: &EqClass, out: &Out) -> Result<Vec<StorePath>> {
        self.get_realisation_paths(eq_class, out).await
    }

    async fn realisation_refs(&self, realisation: &Realisation) -> Result<Vec<Realisation>> {
        self.get_realisation_refs(realisation).await
    }
}

impl LocalStore {
    fn is_store_path<P>(path: P) -> bool
    where
        P: AsRef<Path>,
    {
        path.as_ref().starts_with(&CONFIG.store_dir)
    }

    fn path_to_store(path: &str) -> StorePath {
        unsafe {
            let strip = path.strip_prefix(&Self::store_dir()).unwrap();
            StorePath::from_string(strip[1..].to_string())
        }
    }

    async fn copy_path<P, Q>(&self, src: P, dst: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let src = src.as_ref();
        let dst = dst.as_ref();
        let metadata = fs::metadata(&src).await?;
        let mode = file_type_to_permission(&metadata);
        // if it is a fixed-output derivation src and dst are equal
        if src != dst {
            // delete dst if already exists
            if dst.exists() {
                // set write permissions to delete file
                // TODO: acquire lock before delete
                let mode = metadata.permissions().mode() | 0o200;
                fs::set_permissions(&dst, Permissions::from_mode(mode)).await?;
                if dst.is_dir() {
                    fs::remove_dir_all(&dst).await?;
                } else if dst.is_file() || dst.is_symlink() {
                    fs::remove_file(&dst).await?;
                }
            }
            // if the path is in the store and it is temporary move it
            // otherwise copy it
            if Self::is_store_path(src) {
                fs::rename(&src, &dst).await?;
            } else {
                fs::copy(&src, &dst).await?;
            }
        }
        fs::set_permissions(&dst, Permissions::from_mode(mode)).await?;
        Ok(())
    }

    // TODO: check for cycles
    async fn register_store_obj(&self, obj: StoreObj, refs: Vec<StorePath>) -> Result<()> {
        let mut tx = self.db.begin().await?;
        let referrer = if Self::is_store_obj(&mut tx, &obj.path).await? {
            Self::update_store_obj(&mut tx, &obj).await?
        } else {
            Self::add_store_obj(&mut tx, &obj).await?
        };

        for r in refs {
            let references = Self::get_store_obj_id(&mut tx, &r).await?;
            Self::add_ref(&mut tx, referrer, references).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn register_realisation(
        &self,
        realisation: Realisation,
        eq_refs: Vec<Realisation>,
    ) -> Result<()> {
        // TODO: should we do this in a transaction???
        let referrer = if let Some(referrer) = self.is_realisation(&realisation).await? {
            referrer
        } else {
            self.add_realisation(&realisation).await?
        };
        for eq_ref in eq_refs {
            let references = self.get_realisation_id(&eq_ref).await?;
            self.add_realisation_ref(referrer, references).await?;
        }
        Ok(())
    }
}
