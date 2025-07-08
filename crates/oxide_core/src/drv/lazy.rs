use super::{Drv, IntoDrv};
use crate::drv::DrvPath;
use crate::types::Cow;
use std::cell::{OnceCell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;

struct LazyDrvInner {
    component: RefCell<Option<Box<dyn IntoDrv>>>,
    drv: OnceCell<Rc<Drv>>,
}

impl Debug for LazyDrvInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyDrvInner").finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct LazyDrv(Rc<LazyDrvInner>);

impl LazyDrv {
    pub fn new<T>(component: T) -> Self
    where
        T: IntoDrv + 'static,
    {
        Self(Rc::new(LazyDrvInner {
            component: RefCell::new(Some(Box::new(component))),
            drv: OnceCell::new(),
        }))
    }

    pub fn derive(&self) -> Rc<Drv> {
        Rc::clone(
            self.0
                .drv
                .get_or_init(|| Rc::new(self.0.component.take().unwrap().into_drv())),
        )
    }

    pub fn out<T>(&self, out: T) -> DrvPath
    where
        T: Into<Cow<str>>,
    {
        DrvPath::new(self).out(out)
    }

    pub fn suff<T>(&self, out: T) -> DrvPath
    where
        T: Into<Cow<str>>,
    {
        DrvPath::new(self).suff(out)
    }
}

impl AsRef<LazyDrv> for LazyDrv {
    fn as_ref(&self) -> &LazyDrv {
        self
    }
}
