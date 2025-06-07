use crate::{
    fetchers::fetchurl::FetchUrlDrv,
    pkgs::{fetchers::fetchurl::FetchUrl, hello::Hello, perl::Perl},
    stdenv::{Stdenv, StdenvDrv},
};
use oxide_core::prelude::*;

pub struct AllPkgs {
    pub stdenv: Stdenv,
    pub fetchurl: FetchUrl,
    pub perl: LazyDrv,
    pub hello: LazyDrv,
}

pub fn all_pkgs() -> Box<AllPkgs> {
    let stdenv = Stdenv::new(StdenvDrv {});
    let fetchurl = FetchUrl::new(FetchUrlDrv {
        stdenv: Stdenv::clone(&stdenv),
    });
    let perl = LazyDrv::new(Perl {
        stdenv: Stdenv::clone(&stdenv),
        fetchurl: FetchUrl::clone(&fetchurl),
    });
    let hello = LazyDrv::new(Hello {
        stdenv: Stdenv::clone(&stdenv),
        fetchurl: FetchUrl::clone(&fetchurl),
        perl: LazyDrv::clone(&perl),
    });
    Box::new(AllPkgs {
        stdenv,
        fetchurl,
        perl,
        hello,
    })
}
