use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[allow(non_camel_case_types)]
#[non_exhaustive]
pub enum System {
    #[serde(rename = "x86-64_linux")]
    x86_64_linux,
    #[serde(rename = "i686_linux")]
    i686_linux,
}

impl Display for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                System::x86_64_linux => "x86-64_linux",
                System::i686_linux => "i686_linux",
            }
        )
    }
}
