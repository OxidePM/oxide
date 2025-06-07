use oxide_core::prelude::*;

pub struct StdenvDrv;

impl IntoDrv for StdenvDrv {
    fn into_drv(self: Box<Self>) -> Drv {
        unimplemented!()
    }
}

pub struct StdenvOpt;

#[derive(Clone)]
#[repr(transparent)]
pub struct Stdenv(LazyDrv);

impl IntoDrv for Stdenv {
    fn into_drv(self: Box<Self>) -> Drv {
        unimplemented!()
    }
}

impl Stdenv {
    pub fn new(stdenv: StdenvDrv) -> Self {
        Self(LazyDrv::new(stdenv))
    }

    pub fn derivation(&self, opt: StdenvOpt) -> LazyDrv {
        LazyDrv::new(StdenvParam(opt))
    }
}

pub struct StdenvParam(StdenvOpt);

impl IntoDrv for StdenvParam {
    fn into_drv(self: Box<Self>) -> Drv {
        unimplemented!()
    }
}
