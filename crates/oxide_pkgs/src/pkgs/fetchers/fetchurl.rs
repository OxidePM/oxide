use std::ops::Deref;

use crate::stdenv::Stdenv;
use oxide_core::prelude::*;

pub struct FetchUrlDrv {
    pub stdenv: Stdenv,
}

impl IntoDrv for FetchUrlDrv {
    fn into_drv(self: Box<Self>) -> Drv {
        DrvBuilder::new("fetchurl-0.0.0")
            .builder("/bin/sh")
            .arg("-c")
            .arg(r#"printf "%s\n" "---hello from fetchurl---" $name > $out"#)
            .build()
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct FetchUrl(LazyDrv);

impl Deref for FetchUrl {
    type Target = LazyDrv;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FetchUrl {
    pub fn new(fetchurl: FetchUrlDrv) -> Self {
        Self(LazyDrv::new(fetchurl))
    }

    pub fn fetch<T>(&self, url: T, hash: Hash) -> LazyDrv
    where
        T: Into<Cow<str>>,
    {
        let fetchurl = LazyDrv::clone(&self.0);
        let url = url.into();
        LazyDrv::new(FetchUrlParam {
            fetchurl,
            url,
            hash,
        })
    }
}

pub struct FetchUrlParam {
    fetchurl: LazyDrv,
    url: Cow<str>,
    hash: Hash,
}

impl IntoDrv for FetchUrlParam {
    fn into_drv(self: Box<Self>) -> Drv {
        let name = base_name(&self.url).to_string();
        DrvBuilder::new(name)
            .builder("/bin/sh")
            .arg("-c")
            .arg(r#"printf "%s\n" "---hello from fetchurlparam---" $name $url $hash > $out"#)
            .fixed_hash(self.hash)
            .input("fetchurl", self.fetchurl)
            .input("url", self.url)
            .build()
    }
}
