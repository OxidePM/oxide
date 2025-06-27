use crate::drv::{DEFAULT_OUT, LazyDrv};
use crate::types::Cow;

#[derive(Clone, Debug)]
pub struct DrvPath {
    pub drv: LazyDrv,
    pub out: Cow<str>,
    pub suff: Option<Cow<str>>,
}

impl DrvPath {
    pub fn new(drv: &LazyDrv) -> Self {
        Self {
            drv: LazyDrv::clone(&drv),
            out: Cow::Borrowed(DEFAULT_OUT),
            suff: None,
        }
    }

    pub fn out<T>(mut self, out: T) -> Self
    where
        T: Into<Cow<str>>,
    {
        self.out = out.into();
        self
    }

    pub fn suff<T>(mut self, suff: T) -> Self
    where
        T: Into<Cow<str>>,
    {
        self.suff = Some(suff.into());
        self
    }
}

impl<D> From<D> for DrvPath
where
    D: AsRef<LazyDrv>,
{
    fn from(value: D) -> Self {
        Self::new(value.as_ref())
    }
}
