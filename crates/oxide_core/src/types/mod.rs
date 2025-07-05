use crate::store::StorePath;

/// To rappresent either owned or borrowed data
pub type Cow<T> = std::borrow::Cow<'static, T>;

/// `EqClass` is just an alias for `StorePath`
/// But it does not rappresents a real store path
/// It is a virtual store path used inside of derivations as a temporary output path
/// until the actual output pathh is not known
pub type EqClass = StorePath;

/// Out is just an alias for String
/// It rappresents a derivation output, usually "out"
/// It must not contain spaces
pub type Out = String;
