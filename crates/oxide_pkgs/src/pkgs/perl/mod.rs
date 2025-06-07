use crate::{fetchers::fetchurl::FetchUrl, stdenv::Stdenv};
use oxide_core::prelude::*;

pub struct Perl {
    pub stdenv: Stdenv,
    pub fetchurl: FetchUrl,
}

impl IntoDrv for Perl {
    fn into_drv(self: Box<Self>) -> Drv {
        DrvBuilder::new("perl")
            .out("bin")
            .builder("/bin/sh")
            .arg("-c")
            .arg(r#"echo "---hello from perl---""#)
            .build()
    }
}
