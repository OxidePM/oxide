pub mod globals;

use crate::hash::Hash;
use crate::Cow;
use serde::{Deserialize, Serialize};
use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
    Decode, Sqlite, Type,
};
use std::{fmt::Display, ops::Deref};

/// LENGTH of the base64 encoded hash without algo
pub const HASH_PART_LEN: usize = 64;
/// The bytes of the base64 encoding of the Hash without algo
pub type HashPart = Box<[u8; HASH_PART_LEN]>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
/// A path inside of the store
// to be more general we could have used OsString
// but for our purposes it is useful to be able to
// serialize the data into utf-8 strings
pub struct StorePath(String);

impl StorePath {
    pub fn new(hash: Hash, name: &str) -> Self {
        let mut h = hash.base64();
        h.truncate(HASH_PART_LEN);
        Self(format!("{}-{}", h, name))
    }
}

impl Display for StorePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for StorePath {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Type<Sqlite> for StorePath {
    fn type_info() -> SqliteTypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Encode<'r, Sqlite> for StorePath {
    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'r>>) -> Result<IsNull, BoxDynError>
    where
        Self: Sized,
    {
        args.push(SqliteArgumentValue::Text(Cow::Owned(self.to_string())));
        Ok(IsNull::No)
    }
}

impl<'r> sqlx::Decode<'r, Sqlite> for StorePath {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <String as Decode<Sqlite>>::decode(value)?;
        Ok(StorePath(value))
    }
}
