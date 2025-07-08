mod fetchurl;

use std::collections::HashMap;

pub use fetchurl::*;
use oxide_core::drv::StoreDrv;

pub struct Ctx<'a> {
    pub drv: &'a StoreDrv,
    pub outputs: HashMap<String, String>,
}
