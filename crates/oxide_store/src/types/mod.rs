use oxide_core::{
    hash::Hash,
    store::StorePath,
    types::{EqClass, Out},
};

/// Database ID
pub type ID = u32;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StoreObj {
    pub path: StorePath,
    pub hash: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Realisation {
    pub eq_class: EqClass,
    pub out: Out,
    pub path: StorePath,
}
