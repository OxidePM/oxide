use base64::{Engine, engine::GeneralPurpose, prelude::BASE64_URL_SAFE_NO_PAD};
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
use std::fmt::Display;

pub const BASE64: GeneralPurpose = BASE64_URL_SAFE_NO_PAD;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HashAlgo {
    Sha256,
    Sha512,
}

impl Display for HashAlgo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashAlgo::Sha256 => write!(f, "sha256"),
            HashAlgo::Sha512 => write!(f, "sha512"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Hash {
    Sha256([u8; 32]),
    Sha512(Box<[u8; 64]>),
}

impl Hash {
    pub fn placeholder() -> Hash {
        Hash::Sha512(Box::new([0; 64]))
    }

    pub fn algo(&self) -> HashAlgo {
        match self {
            Hash::Sha256(_) => HashAlgo::Sha256,
            Hash::Sha512(_) => HashAlgo::Sha512,
        }
    }

    pub fn digest_as_bytes(&self) -> &[u8] {
        match self {
            Hash::Sha256(digest) => digest,
            Hash::Sha512(digest) => digest.as_ref(),
        }
    }

    pub fn base64(&self) -> String {
        BASE64.encode(self.digest_as_bytes())
    }

    pub fn base64_with_algo(&self) -> String {
        format!("{}:{}", self.algo(), self.base64())
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.base64_with_algo())
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.base64_with_algo().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct HashVisitor;
        impl Visitor<'_> for HashVisitor {
            type Value = Hash;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Hash")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Hash::try_from(v).map_err(|_| {
                    de::Error::invalid_value(de::Unexpected::Str(v), &"a well formed hash")
                })
            }
        }
        deserializer.deserialize_str(HashVisitor)
    }
}

#[derive(Clone, Debug)]
pub struct ParseHashError;

impl TryFrom<&str> for Hash {
    type Error = ParseHashError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        macro_rules! parse {
            ($prefix:literal, $hash:expr) => {
                if let Some(v) = value.strip_prefix($prefix) {
                    return Ok($hash(
                        BASE64
                            .decode(v)
                            .map_err(|_| ParseHashError)?
                            .try_into()
                            .map_err(|_| ParseHashError)?,
                    ));
                }
            };
        }
        parse!("sha256:", Hash::Sha256);
        parse!("sha512:", Hash::Sha512);
        Err(ParseHashError)
    }
}
