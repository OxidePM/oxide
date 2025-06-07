use crate::models::ID;
use oxide_core::{hash::Hash, store::StorePath};

#[derive(sqlx::FromRow)]
pub struct PathInfo {
    pub id: ID,
    pub path: StorePath,
    pub hash: Hash,
}
