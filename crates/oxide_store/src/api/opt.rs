use oxide_core::{hash::HashAlgo, store::HashPart, store::StorePath};
use std::collections::HashMap;

pub struct EqRefs {
    pub eq_class: StorePath,
    pub refs: Vec<EqRef>,
}

pub struct EqRef {
    pub path: StorePath,
    pub eq_class: StorePath,
}

pub struct Opt {
    pub algo: HashAlgo,
    pub refs: Vec<StorePath>,
    pub eq_refs: Option<EqRefs>,
    pub name: String,
    pub rewrites: HashMap<HashPart, HashPart>,
    pub self_hash: Option<HashPart>,
}
