use crate::types::Realisation;
use oxide_core::{
    hash::HashAlgo,
    store::{HashPart, StorePath},
    types::{EqClass, Out},
};
use std::collections::HashMap;

pub struct EqRefs {
    pub eq_class: EqClass,
    pub out: Out,
    pub refs: Vec<Realisation>,
}

pub struct Opt {
    pub algo: HashAlgo,
    pub refs: Vec<StorePath>,
    pub eq_refs: Option<EqRefs>,
    pub name: String,
    pub rewrites: HashMap<HashPart, HashPart>,
    pub self_hash: Option<HashPart>,
}
