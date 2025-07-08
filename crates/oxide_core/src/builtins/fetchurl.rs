use crate::{
    drv::{Drv, DrvBuilder, IntoDrv},
    hash::Hash,
    types::Cow,
    utils::to_base_name,
};

pub struct FetchUrl {
    pub name: Option<Cow<str>>,
    pub url: Cow<str>,
    pub hash: Hash,
    pub unpack: bool,
    pub executable: bool,
}

impl IntoDrv for FetchUrl {
    fn into_drv(self) -> Drv {
        let name = self
            .name
            .unwrap_or_else(|| to_base_name(self.url.to_string()).into());
        DrvBuilder::new()
            .name(name)
            .fixed_hash(self.hash)
            .input("url", self.url)
            .input_if("unpack", self.unpack.then_some("1"))
            .input_if("executable", self.executable.then_some("1"))
            .build()
    }
}
