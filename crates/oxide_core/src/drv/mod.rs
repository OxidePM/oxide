mod lazy;
mod path;
mod store;

pub use lazy::*;
pub use path::*;
pub use store::*;

use crate::expr::Expr;
use crate::hash::Hash;
use crate::system::System;
use crate::types::Cow;
use crate::utils::current_system;
use std::collections::HashMap;

pub const DRV_EXT: &str = ".drv";
pub const DEFAULT_OUT: &str = "out";

pub trait IntoDrv {
    fn into_drv(self: Box<Self>) -> Drv;
}

#[derive(Clone, Debug)]
pub struct Drv {
    pub name: Cow<str>,
    pub outputs: Vec<Cow<str>>,
    pub fixed_hash: Option<Hash>,
    pub system: System,
    pub inputs: HashMap<String, Expr>,
    pub builder: Expr,
    pub args: Vec<Expr>,
}

impl IntoDrv for Drv {
    fn into_drv(self: Box<Self>) -> Drv {
        *self
    }
}

pub struct DrvBuilder {
    name: Option<Cow<str>>,
    outputs: Vec<Cow<str>>,
    fixed_hash: Option<Hash>,
    system: Option<System>,
    inputs: HashMap<String, Expr>,
    builder: Option<Expr>,
    args: Vec<Expr>,
}

impl DrvBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            outputs: Vec::new(),
            fixed_hash: None,
            system: None,
            inputs: HashMap::new(),
            builder: None,
            args: Vec::new(),
        }
    }

    pub fn name<T>(mut self, name: T) -> Self
    where
        T: Into<Cow<str>>,
    {
        self.name = Some(name.into());
        self
    }

    pub fn out<T>(mut self, out: T) -> Self
    where
        T: Into<Cow<str>>,
    {
        self.outputs.push(out.into());
        self
    }

    pub fn fixed_hash(mut self, hash: Hash) -> Self {
        self.fixed_hash = Some(hash);
        self
    }

    pub fn system(mut self, system: System) -> Self {
        self.system = Some(system);
        self
    }

    pub fn input<K, V>(mut self, key: K, expr: V) -> Self
    where
        K: Into<String>,
        V: Into<Expr>,
    {
        self.inputs.insert(key.into(), expr.into());
        self
    }

    pub fn input_if<K, V>(mut self, key: K, expr: Option<V>) -> Self
    where
        K: Into<String>,
        V: Into<Expr>,
    {
        if let Some(expr) = expr {
            self.inputs.insert(key.into(), expr.into());
        }
        self
    }

    pub fn builder<T>(mut self, builder: T) -> Self
    where
        T: Into<Expr>,
    {
        self.builder = Some(builder.into());
        self
    }

    pub fn arg<T>(mut self, arg: T) -> Self
    where
        T: Into<Expr>,
    {
        self.args.push(arg.into());
        self
    }

    pub fn build(self) -> Drv {
        Drv {
            name: self.name.expect("name must be provided"),
            outputs: if self.outputs.is_empty() {
                vec![DEFAULT_OUT.into()]
            } else {
                self.outputs
            },
            fixed_hash: self.fixed_hash,
            system: self.system.unwrap_or(current_system()),
            inputs: self.inputs,
            builder: self.builder.expect("builder must be provided"),
            args: self.args,
        }
    }
}
