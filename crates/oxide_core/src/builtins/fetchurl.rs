use crate::{
    drv::{Drv, DrvBuilder, IntoDrv},
    hash::Hash,
    types::Cow,
    utils::to_base_name,
};

pub struct FetchUrl {
    name: Option<Cow<str>>,
    url: Cow<str>,
    hash: Hash,
    unpack: bool,
    executable: bool,
}

impl IntoDrv for FetchUrl {
    fn into_drv(self: Box<Self>) -> Drv {
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
