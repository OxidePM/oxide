use anyhow::{bail, Result};
use oxide_core::{
    drv::{LazyDrv, StoreDrv, DRV_EXT},
    expr::Expr,
    hash::{Hash, HashAlgo},
    store::StorePath,
    utils::file_name,
};
use oxide_store::{
    api::{Opt, Store},
    hash::utils::make_path,
    stores::local,
};
use std::collections::{BTreeSet, HashMap};
use std::{collections::BTreeMap, io::Cursor};
use tokio::io::BufReader;

pub async fn instantiate(drv: &LazyDrv) -> Result<(StoreDrv, StorePath)> {
    let drv = (*drv.derive()).clone();
    let mut input_drvs: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
    let mut input_srcs = BTreeSet::new();
    let mut envs = BTreeMap::new();
    let mut args = Vec::new();
    let store = local::LocalStore::new().await?;
    // TODO: should we add builder and args???
    let mut inputs = drv.inputs;
    inputs.insert("name".to_string(), Expr::Str(drv.name.clone()));
    inputs.insert(
        "outputs".to_string(),
        Expr::Array(
            drv.outputs
                .iter()
                .cloned()
                .map(|out| Expr::Str(out))
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    if let Some(ref fixed_hash) = drv.fixed_hash {
        inputs.insert(
            "fixed_hash".to_string(),
            Expr::Str(fixed_hash.base64_with_algo().into()),
        );
    }
    inputs.insert(
        "system".to_string(),
        Expr::Str(drv.system.to_string().into()),
    );
    for (k, v) in inputs {
        let ps = Box::pin(process_bindings(&store, v)).await?;
        for (out, outputs) in ps.drvs {
            input_drvs.entry(out).or_default().extend(outputs);
        }
        input_srcs.extend(ps.srcs);
        envs.insert(k, ps.res.join(" "));
    }
    let builder = Box::pin(process_bindings(&store, drv.builder))
        .await?
        .res
        .join(" ");
    for arg in drv.args {
        let ps = Box::pin(process_bindings(&store, arg)).await?;
        args.extend(ps.res);
    }
    let eq_classes = drv
        .outputs
        .into_iter()
        .enumerate()
        .map(|(i, out)| {
            let name = if i == 0 {
                drv.name.as_ref()
            } else {
                &format!("{}-{}", drv.name, out)
            };
            // TODO: eq_class
            let eq_class = make_path(Hash::Sha512(Box::new([0; 64])), name);
            (out.into_owned(), eq_class)
        })
        .collect::<BTreeMap<String, StorePath>>();
    envs.extend(
        eq_classes
            .clone()
            .into_iter()
            .map(|(out, eq_class)| (out, eq_class.to_string())),
    );
    let fixed_hash = drv.fixed_hash;
    let system = drv.system;
    let mut refs = Vec::new();
    refs.extend(input_drvs.clone().into_iter().map(|(d, _)| d));
    refs.extend(input_srcs.clone());
    let d = StoreDrv {
        eq_classes,
        fixed_hash,
        input_drvs,
        input_srcs,
        system,
        builder,
        args,
        envs,
    };
    let p = store
        .add_to_store_buff(
            drv_to_buff(&d),
            Opt {
                algo: HashAlgo::Sha512,
                refs,
                // TODO: eq_refs
                eq_refs: None,
                name: drv.name.into_owned() + DRV_EXT,
                rewrites: HashMap::new(),
                self_hash: None,
            },
        )
        .await?;
    Ok((d, p))
}

fn drv_to_buff(drv: &StoreDrv) -> BufReader<Cursor<String>> {
    let drv_str = toml::to_string_pretty(&drv).unwrap();
    BufReader::new(Cursor::new(drv_str))
}

#[derive(Default)]
struct BindRes {
    drvs: BTreeMap<StorePath, BTreeSet<String>>,
    srcs: BTreeSet<StorePath>,
    res: Vec<String>,
}

async fn process_bindings(store: &local::LocalStore, expr: Expr) -> Result<BindRes> {
    Ok(match expr {
        Expr::Str(s) => BindRes {
            res: vec![s.into_owned()],
            ..Default::default()
        },
        Expr::Path(path) => {
            let p = store
                .add_to_store(
                    &path,
                    Opt {
                        algo: HashAlgo::Sha512,
                        refs: Vec::new(),
                        eq_refs: None,
                        name: file_name(&path),
                        rewrites: HashMap::new(),
                        self_hash: None,
                    },
                )
                .await?;
            BindRes {
                srcs: BTreeSet::from([p.clone()]),
                res: vec![p.to_string()],
                ..Default::default()
            }
        }
        Expr::Drv(drv_path) => {
            let drv = drv_path.drv;
            let out = drv_path.out.into_owned();
            let (d, p) = Box::pin(instantiate(&drv)).await?;
            let outp = if let Some(out) = d.eq_classes.get(&out) {
                out.to_string()
            } else {
                bail!("invalid output: {} not present", out);
            };
            BindRes {
                drvs: BTreeMap::from([(p, BTreeSet::from([out]))]),
                res: vec![outp],
                ..Default::default()
            }
        }
        Expr::Array(array) => {
            let mut drvs: BTreeMap<_, BTreeSet<String>> = BTreeMap::new();
            let mut srcs = BTreeSet::new();
            let mut res = Vec::new();
            for e in array.into_owned() {
                let ps = Box::pin(process_bindings(store, e)).await?;
                for (out, outputs) in ps.drvs {
                    drvs.entry(out).or_default().extend(outputs);
                }
                srcs.extend(ps.srcs);
                res.extend(ps.res);
            }
            BindRes { drvs, srcs, res }
        }
    })
}
