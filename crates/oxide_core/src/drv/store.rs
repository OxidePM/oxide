use crate::hash::Hash;
use crate::{store::StorePath, system::System};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoreDrv {
    pub eq_classes: BTreeMap<String, StorePath>,
    pub fixed_hash: Option<Hash>,
    pub input_drvs: BTreeMap<StorePath, BTreeSet<String>>,
    pub input_srcs: BTreeSet<StorePath>,
    pub system: System,
    pub builder: String,
    pub args: Vec<String>,
    pub envs: BTreeMap<String, String>,
}
