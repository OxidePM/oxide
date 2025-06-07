use crate::api::{Opt, Store, CONFIG};
use crate::hash::hash;
use crate::hash::utils::make_path;
use crate::models::{PathInfo, ID};
use crate::os::lock::{LockMode, PathLock};
use crate::utils::tempfile::is_temp;
use crate::utils::{add_lock_ext, is_valid_name};
use anyhow::{bail, Result};
use oxide_core::store::StorePath;
use sqlx::migrate::Migrator;
use sqlx::SqlitePool;
use std::cell::LazyCell;
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
        let db_path = format!("{}/sqlite.db", db_dir);
        let migrations_dir = format!("{}/migrations", db_dir);
        Self {
            db_dir,
            db_path,
            migrations_dir,
        }
    }
}

pub const LOCAL_STORE_CONFIG: LazyCell<LocalStoreConfig> =
    LazyCell::new(|| LocalStoreConfig::new());

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
    async fn add_to_store<P>(&self, path: P, opt: Opt) -> Result<StorePath>
    where
        P: AsRef<Path>,
    {
        if !is_valid_name(&opt.name) {
            bail!("invalid name: {}", opt.name);
        }
        // TODO: add fix flag
        let fix = false;

        let hash = hash(&path, opt.algo, &opt.rewrites, opt.self_hash.as_ref()).await?;
        let store_path = make_path(hash.clone(), &opt.name);
        if !self.valid(&store_path).await? || fix {
            // TODO: better logging
            println!("add to store: {} {}", opt.name, store_path.to_string());
            let real_path = self.real_store_path(&store_path);
            let lock_file = add_lock_ext(&real_path);
            let lock = PathLock::lock(lock_file, LockMode::Write)?;
            if !self.valid(&store_path).await? || fix {
                self.copy_path(&path, &real_path).await?;
                let info = PathInfo {
                    id: 0,
                    path: store_path.clone(),
                    hash,
                };
                self.register_valid_path(info, &opt).await?;
            }
            lock.unlock();
        }
        self.set_eq_refs(&opt).await?;

        Ok(store_path)
    }

    fn real_store_path(&self, path: &StorePath) -> String {
        format!("{}/{}", CONFIG.store_dir, path)
    }
}

impl LocalStore {
    fn is_temp<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        path.as_ref().starts_with(&CONFIG.store_dir) && is_temp(path)
    }

    async fn copy_path<P, Q>(&self, src: P, dst: Q) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        // delete dst if already exists
        if dst.as_ref().is_dir() {
            fs::remove_dir_all(&dst).await?;
        } else if dst.as_ref().is_file() || dst.as_ref().is_symlink() {
            fs::remove_file(&dst).await?;
        }
        // if the path is in the store and it is temporary move it
        // else copy it
        if self.is_temp(&src) {
            fs::rename(&src, &dst).await?;
        } else {
            fs::copy(&src, &dst).await?;
        }
        Ok(())
    }

    pub async fn valid(&self, path: &StorePath) -> Result<bool> {
        Ok(sqlx::query("SELECT id FROM path WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.db)
            .await?
            .is_some())
    }

    async fn is_valid_path(
        tx: &mut sqlx::SqliteTransaction<'static>,
        path: &StorePath,
    ) -> Result<bool> {
        Ok(sqlx::query("SELECT id FROM path WHERE path = ?")
            .bind(path)
            .fetch_optional(&mut **tx)
            .await?
            .is_some())
    }

    async fn get_valid_path(
        tx: &mut sqlx::SqliteTransaction<'static>,
        path: &StorePath,
    ) -> Result<PathInfo> {
        Ok(sqlx::query_as("SELECT * FROM path WHERE path = ?")
            .bind(path)
            .fetch_one(&mut **tx)
            .await?)
    }

    async fn add_valid_path(
        tx: &mut sqlx::SqliteTransaction<'static>,
        info: PathInfo,
    ) -> Result<ID> {
        let (id, ..): (ID,) =
            sqlx::query_as("INSERT INTO path (path, hash) VALUES (?, ?) RETURNING id")
                .bind(info.path)
                .bind(info.hash)
                .fetch_one(&mut **tx)
                .await?;
        Ok(id)
    }

    async fn update_valid_path(
        tx: &mut sqlx::SqliteTransaction<'static>,
        info: PathInfo,
    ) -> Result<ID> {
        let (id, ..): (ID,) =
            sqlx::query_as("UPDATE path SET hash = ? WHERE path = ? RETURNING id")
                .bind(info.hash)
                .bind(info.path)
                .fetch_one(&mut **tx)
                .await?;
        Ok(id)
    }

    async fn add_ref(
        tx: &mut sqlx::SqliteTransaction<'static>,
        referrer: ID,
        references: ID,
    ) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO ref (referrer, reference) VALUES (?, ?)")
            .bind(referrer)
            .bind(references)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    // TODO: check for cycles
    async fn register_valid_path(&self, info: PathInfo, opt: &Opt) -> Result<()> {
        let mut tx = self.db.begin().await?;
        let referrer = if Self::is_valid_path(&mut tx, &info.path).await? {
            Self::update_valid_path(&mut tx, info).await?
        } else {
            Self::add_valid_path(&mut tx, info).await?
        };

        for r in opt.refs.iter() {
            let references = Self::get_valid_path(&mut tx, r).await?.id;
            Self::add_ref(&mut tx, referrer, references).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    // TODO
    async fn set_eq_refs(&self, opt: &Opt) -> Result<()> {
        _ = opt;
        Ok(())
    }
}
