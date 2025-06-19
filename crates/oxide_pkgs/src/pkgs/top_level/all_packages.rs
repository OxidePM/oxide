use std::collections::HashMap;

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

// TODO: make it more ergonomic
pub fn all_pkgs() -> (HashMap<String, LazyDrv>, Box<AllPkgs>) {
    let mut pkgs = HashMap::new();
    let stdenv = Stdenv::new(StdenvDrv {});
    pkgs.insert("stdenv".to_string(), LazyDrv::clone(&*stdenv));
    let fetchurl = FetchUrl::new(FetchUrlDrv {
        stdenv: Stdenv::clone(&stdenv),
    });
    pkgs.insert("fetchurl".to_string(), LazyDrv::clone(&*fetchurl));
    let perl = LazyDrv::new(Perl {
        stdenv: Stdenv::clone(&stdenv),
        fetchurl: FetchUrl::clone(&fetchurl),
    });
    pkgs.insert("perl".to_string(), LazyDrv::clone(&perl));
    let hello = LazyDrv::new(Hello {
        stdenv: Stdenv::clone(&stdenv),
        fetchurl: FetchUrl::clone(&fetchurl),
        perl: LazyDrv::clone(&perl),
    });
    pkgs.insert("hello".to_string(), LazyDrv::clone(&hello));
    (
        pkgs,
        Box::new(AllPkgs {
            stdenv,
            fetchurl,
            perl,
            hello,
        }),
    )
}
