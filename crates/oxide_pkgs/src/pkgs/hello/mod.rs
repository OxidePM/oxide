use crate::{fetchers::fetchurl::FetchUrl, stdenv::Stdenv};
use oxide_core::prelude::*;

pub struct Hello {
    pub fetchurl: FetchUrl,
    pub stdenv: Stdenv,
    pub perl: LazyDrv,
}

impl IntoDrv for Hello {
    fn into_drv(self: Box<Self>) -> Drv {
        DrvBuilder::new("hello-0.0.1")
            .builder(local_file!("builder.sh"))
            .input("perl", self.perl.out("bin").suff("/bin/perl"))
            .input(
                "src",
                self.fetchurl.fetch(
                    "https://fake.url.com/test2/testurl-1.0",
                    hash!("sha256:abcdefghabcdefghabcdefghabcdefghabcdefghabc"),
                ),
            )
            .input(
                "patches",
                vec![
                    local_file!("patches/abcd.patch"),
                    local_file!("patches/efgh.patch"),
                ],
            )
            .input(
                // TODO: this is ugly how do we simplify the interface
                // I do not want to call into every time
                "usrs",
                vec![
                    "alice".into(),
                    "bob".into(),
                    vec!["eve".into(), "john \"doe\"".into()].into(),
                ],
            )
            .build()
    }
}
