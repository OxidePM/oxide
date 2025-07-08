use crate::builtins::BUILTIN_PREFIX;
use crate::hash::Hash;
use crate::types::{EqClass, Out};
use crate::utils::to_base_name;
use crate::{store::StorePath, system::System};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct StoreDrv {
    pub eq_classes: BTreeMap<Out, EqClass>,
    pub fixed_hash: Option<Hash>,
    pub input_drvs: BTreeMap<StorePath, BTreeSet<Out>>,
    pub input_srcs: BTreeSet<StorePath>,
    pub system: System,
    pub builder: String,
    pub args: Vec<String>,
    pub envs: BTreeMap<String, String>,
}

impl StoreDrv {
    pub fn builtin(&self) -> Option<&str> {
        self.builder.strip_prefix(BUILTIN_PREFIX)
    }
}

impl<'de> Deserialize<'de> for StoreDrv {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StoreDrvRaw {
            eq_classes: BTreeMap<Out, String>,
            fixed_hash: Option<Hash>,
            input_drvs: BTreeMap<String, BTreeSet<Out>>,
            input_srcs: BTreeSet<String>,
            system: System,
            builder: String,
            args: Vec<String>,
            envs: BTreeMap<String, String>,
        }

        let raw = StoreDrvRaw::deserialize(deserializer)?;
        let eq_classes = raw
            .eq_classes
            .into_iter()
            .map(|(k, v)| {
                let base_name = to_base_name(v);
                let eq_class = unsafe { StorePath::from_string(base_name) };
                (k, eq_class)
            })
            .collect();
        let input_drvs = raw
            .input_drvs
            .into_iter()
            .map(|(k, v)| {
                let base_name = to_base_name(k);
                (unsafe { StorePath::from_string(base_name) }, v)
            })
            .collect();
        let input_srcs = raw
            .input_srcs
            .into_iter()
            .map(|s| {
                let base_name = to_base_name(s);
                unsafe { StorePath::from_string(base_name) }
            })
            .collect();

        Ok(StoreDrv {
            eq_classes,
            fixed_hash: raw.fixed_hash,
            input_drvs,
            input_srcs,
            system: raw.system,
            builder: raw.builder,
            args: raw.args,
            envs: raw.envs,
        })
    }
}

pub struct DrvSerializer<'a, F> {
    pub full_path: F,
    pub drv: &'a StoreDrv,
}

impl<F> Serialize for DrvSerializer<'_, F>
where
    F: Fn(&StorePath) -> String,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_struct("StoreDrv", 8)?;
        let eq_classes = &self
            .drv
            .eq_classes
            .iter()
            .map(|(k, v)| (k, (self.full_path)(v)))
            .collect::<BTreeMap<_, _>>();
        state.serialize_field("eq_classes", &eq_classes)?;
        state.serialize_field("fixed_hash", &self.drv.fixed_hash)?;
        let input_drvs = self
            .drv
            .input_drvs
            .iter()
            .map(|(k, v)| ((self.full_path)(k), v))
            .collect::<BTreeMap<_, _>>();
        state.serialize_field("input_drvs", &input_drvs)?;
        let input_srcs = self
            .drv
            .input_srcs
            .iter()
            .map(|p| (self.full_path)(p))
            .collect::<BTreeSet<_>>();
        state.serialize_field("input_srcs", &input_srcs)?;
        state.serialize_field("system", &self.drv.system)?;
        state.serialize_field("builder", &self.drv.builder)?;
        state.serialize_field("args", &self.drv.args)?;
        state.serialize_field("envs", &self.drv.envs)?;

        state.end()
    }
}
