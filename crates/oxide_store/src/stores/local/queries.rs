use super::LocalStore;
use crate::api::Store;
use crate::types::{Realisation, StoreObj, ID};
use anyhow::Result;
use oxide_core::store::StorePath;
use oxide_core::types::{EqClass, Out};
use sqlx::Row;

impl LocalStore {
    pub(super) async fn valid(&self, path: &StorePath) -> Result<bool> {
        Ok(sqlx::query("SELECT id FROM store_obj WHERE path = ?")
            .bind(Self::store_path(path))
            .fetch_optional(&self.db)
            .await?
            .is_some())
    }

    pub(super) async fn is_store_obj(
        tx: &mut sqlx::SqliteTransaction<'static>,
        path: &StorePath,
    ) -> Result<bool> {
        Ok(sqlx::query("SELECT id FROM store_obj WHERE path = ?")
            .bind(Self::store_path(path))
            .fetch_optional(&mut **tx)
            .await?
            .is_some())
    }

    pub(super) async fn get_store_obj_id(
        tx: &mut sqlx::SqliteTransaction<'static>,
        path: &StorePath,
    ) -> Result<ID> {
        let (id, ..): (ID,) = sqlx::query_as("SELECT id FROM store_obj WHERE path = ?")
            .bind(Self::store_path(path))
            .fetch_one(&mut **tx)
            .await?;
        Ok(id)
    }

    pub(super) async fn add_store_obj(
        tx: &mut sqlx::SqliteTransaction<'static>,
        obj: &StoreObj,
    ) -> Result<ID> {
        let (id, ..): (ID,) =
            sqlx::query_as("INSERT INTO store_obj (path, hash) VALUES (?, ?) RETURNING id")
                .bind(Self::store_path(&obj.path))
                .bind(obj.hash.base64_with_algo())
                .fetch_one(&mut **tx)
                .await?;
        Ok(id)
    }

    pub(super) async fn update_store_obj(
        tx: &mut sqlx::SqliteTransaction<'static>,
        obj: &StoreObj,
    ) -> Result<ID> {
        let (id, ..): (ID,) =
            sqlx::query_as("UPDATE store_obj SET hash = ? WHERE path = ? RETURNING id")
                .bind(obj.hash.base64_with_algo())
                .bind(Self::store_path(&obj.path))
                .fetch_one(&mut **tx)
                .await?;
        Ok(id)
    }

    pub(super) async fn add_ref(
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

    pub(super) async fn is_realisation(&self, realisation: &Realisation) -> Result<Option<ID>> {
        let id: Option<(ID,)> = sqlx::query_as(
            r#"SELECT r.id
            FROM realisation r
            JOIN store_obj o ON r.obj = o.id
            WHERE r.eq_class = ? AND r.out = ? AND o.path = ?"#,
        )
        .bind(&*realisation.eq_class)
        .bind(&realisation.out)
        .bind(Self::store_path(&realisation.path))
        .fetch_optional(&self.db)
        .await?;
        Ok(id.map(|id| id.0))
    }

    pub(super) async fn add_realisation(&self, realisation: &Realisation) -> Result<ID> {
        let (id, ..): (ID,) = sqlx::query_as(
            r#"INSERT INTO realisation (eq_class, out, obj) VALUES
            (?, ?, (SELECT id FROM store_obj WHERE path = ?))
            RETURNING id"#,
        )
        .bind(&*realisation.eq_class)
        .bind(&realisation.out)
        .bind(Self::store_path(&realisation.path))
        .fetch_one(&self.db)
        .await?;
        Ok(id)
    }

    pub(super) async fn get_realisation_id(&self, realisation: &Realisation) -> Result<ID> {
        let (id, ..): (ID,) = sqlx::query_as(
            r#"
                SELECT r.id
                FROM realisation r
                JOIN store_obj o ON r.obj = o.id
                WHERE r.eq_class = ? AND r.out = ? AND o.path = ?
                "#,
        )
        .bind(&*realisation.eq_class)
        .bind(&realisation.out)
        .bind(Self::store_path(&realisation.path))
        .fetch_one(&self.db)
        .await?;
        Ok(id)
    }

    pub(super) async fn add_realisation_ref(&self, referrer: ID, references: ID) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO realisation_ref (referrer, reference) VALUES (?, ?)")
            .bind(referrer)
            .bind(references)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub(super) async fn get_realisation_paths(
        &self,
        eq_class: &EqClass,
        out: &Out,
    ) -> Result<Vec<StorePath>> {
        let rows = sqlx::query(
            r#"
            SELECT path 
            FROM realisation r
            JOIN store_obj o ON o.id = r.obj
            WHERE r.eq_class = ? AND r.out = ?
            "#,
        )
        .bind(Self::store_path(&eq_class))
        .bind(out)
        .fetch_all(&self.db)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| Self::path_to_store(row.get(0)))
            .collect())
    }

    pub(super) async fn get_realisation_refs(
        &self,
        realisation: &Realisation,
    ) -> Result<Vec<Realisation>> {
        let referrer = self.get_realisation_id(realisation).await?;
        let rows = sqlx::query(
            r#"
            SELECT r.eq_class, r.out, o.path
            FROM realisation r
            JOIN realisation_ref ref ON ref.references = r.id
            JOIN store_obj o ON o.id = r.obj
            WHERE ref.referrer = ?
            "#,
        )
        .bind(referrer)
        .fetch_all(&self.db)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| Realisation {
                eq_class: Self::path_to_store(row.get(0)),
                out: row.get(1),
                path: Self::path_to_store(row.get(2)),
            })
            .collect())
    }

    fn path_to_store(path: String) -> StorePath {
        unsafe {
            StorePath::from_string(path.strip_prefix(&Self::store_dir()).unwrap().to_string())
        }
    }
}
