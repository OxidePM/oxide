use crate::drv::DrvPath;
use crate::Cow;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum Expr {
    Str(Cow<str>),
    Path(Cow<Path>),
    Drv(DrvPath),
    Array(Cow<[Expr]>),
}

impl From<&'static str> for Expr {
    fn from(value: &'static str) -> Self {
        Self::Str(Cow::Borrowed(value))
    }
}

impl From<String> for Expr {
    fn from(value: String) -> Self {
        Self::Str(Cow::Owned(value))
    }
}

impl From<Cow<str>> for Expr {
    fn from(value: Cow<str>) -> Self {
        Self::Str(value)
    }
}

impl From<&'static Path> for Expr {
    fn from(value: &'static Path) -> Self {
        Self::Path(Cow::Borrowed(value))
    }
}

impl From<PathBuf> for Expr {
    fn from(value: PathBuf) -> Self {
        Self::Path(Cow::Owned(value))
    }
}

impl From<Cow<Path>> for Expr {
    fn from(value: Cow<Path>) -> Self {
        Self::Path(value)
    }
}

impl<T> From<T> for Expr
where
    T: Into<DrvPath>,
{
    fn from(value: T) -> Self {
        Self::Drv(value.into())
    }
}

impl From<&'static [Expr]> for Expr {
    fn from(value: &'static [Expr]) -> Self {
        Self::Array(Cow::Borrowed(value))
    }
}

impl From<Vec<Expr>> for Expr {
    fn from(value: Vec<Expr>) -> Self {
        Self::Array(Cow::Owned(value))
    }
}

impl From<Cow<[Expr]>> for Expr {
    fn from(value: Cow<[Expr]>) -> Self {
        Self::Array(value)
    }
}
