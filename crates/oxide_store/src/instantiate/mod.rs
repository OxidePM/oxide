use crate::{
    api::{Opt, Store},
    hash::utils::make_path,
    utils::is_valid_name,
};
use anyhow::{Result, bail};
use oxide_core::{
    drv::{DEFAULT_OUT, DRV_EXT, Drv, DrvSerializer, LazyDrv, StoreDrv},
    expr::Expr,
    hash::{Hash, HashAlgo},
    store::StorePath,
    types::Out,
    utils::file_name,
};
use sha2::{Digest, Sha512};
use std::{
    collections::{BTreeMap, HashSet},
    io::Cursor,
};
use std::{
    collections::{BTreeSet, HashMap},
    sync::{LazyLock, Mutex},
};
use tokio::io::BufReader;

pub const NAME_KEY: &str = "name";
pub const OUTPUTS_KEY: &str = "outputs";
pub const FIXED_HASH_KEY: &str = "fixed_hash";
pub const SYSTEM_KEY: &str = "system";
pub const BUILDER_KEY: &str = "builder";

pub fn is_valid_drv(drv: &Drv) -> Result<()> {
    if drv.name.ends_with(DRV_EXT) {
        bail!(
            "invalid name {}: derivation names cannot end with {}",
            drv.name,
            DRV_EXT
        )
    } else if !is_valid_name(&drv.name) {
        bail!(
            "invalid name {}: derivation name can only contain alfanumeric chars and '.', '-', '_'",
            drv.name,
        )
    } else if drv.fixed_hash.is_some() && drv.outputs != [DEFAULT_OUT] {
        bail!(
            "fixed-output derivations must contain a single output called {}",
            DEFAULT_OUT
        )
    }
    Ok(())
}

// TODO: move most of the derivation logic to another file
pub async fn instantiate<S>(store: &S, drv: &LazyDrv) -> Result<(StoreDrv, StorePath)>
where
    S: Store,
{
    let drv = (*drv.derive()).clone();
    is_valid_drv(&drv)?;
    let mut input_drvs: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
    let mut input_srcs = BTreeSet::new();
    let mut envs = default_envs(&drv);
    let mut args = Vec::new();
    let inputs = drv.inputs;
    for (k, v) in inputs {
        let ps = Box::pin(process_expr(store, v)).await?;
        for (out, outputs) in ps.drvs {
            input_drvs.entry(out).or_default().extend(outputs);
        }
        input_srcs.extend(ps.srcs);
        envs.insert(k, ps.res.join(" "));
    }
    let builder = {
        let ps = Box::pin(process_expr(store, drv.builder)).await?;
        let builder = ps.res.join(" ");
        for (out, outputs) in ps.drvs {
            input_drvs.entry(out).or_default().extend(outputs);
        }
        input_srcs.extend(ps.srcs);
        envs.insert(BUILDER_KEY.to_string(), builder.clone());
        builder
    };
    for arg in drv.args {
        let ps = Box::pin(process_expr(store, arg)).await?;
        for (out, outputs) in ps.drvs {
            input_drvs.entry(out).or_default().extend(outputs);
        }
        input_srcs.extend(ps.srcs);
        args.extend(ps.res);
        // no need to add args to envs
        // use $@ instead
    }
    let fixed_hash = drv.fixed_hash;
    let system = drv.system;
    let refs = {
        let mut refs = HashSet::new();
        refs.extend(input_drvs.keys().cloned());
        refs.extend(input_srcs.iter().cloned());
        refs
    };
    // empty eq_classes
    let eq_classes = drv
        .outputs
        .iter()
        .cloned()
        .map(|out| (out.into_owned(), unsafe { StorePath::empty() }))
        .collect();
    // empty eq_class for each out in outputs
    envs.extend(
        drv.outputs
            .iter()
            .cloned()
            .map(|out| (out.into_owned(), String::new())),
    );
    let mut d = StoreDrv {
        eq_classes,
        fixed_hash,
        input_drvs,
        input_srcs,
        system,
        builder,
        args,
        envs,
    };
    let drv_hash = hash_drv(store, d.clone()).await?;
    // actual eq_classes
    d.eq_classes = drv
        .outputs
        .into_iter()
        .enumerate()
        .map(|(i, out)| {
            let name = if i == 0 {
                drv.name.as_ref()
            } else {
                &format!("{}-{}", drv.name, out)
            };
            let eq_class = make_path(&drv_hash, name);
            (out.into_owned(), eq_class)
        })
        .collect();
    // actual (output, eq_class) in envs
    d.envs.extend(
        d.eq_classes
            .iter()
            .map(|(out, eq_class)| (out.clone(), S::store_path(eq_class))),
    );
    let p = store
        .add_to_store_buff(
            drv_to_buff::<S>(&d)?,
            Opt {
                algo: HashAlgo::Sha512,
                refs,
                eq_refs: None,
                name: drv.name.into_owned() + DRV_EXT,
                rewrites: HashMap::new(),
                self_hash: None,
            },
        )
        .await?;
    Ok((d, p))
}

fn default_envs(drv: &Drv) -> BTreeMap<String, String> {
    // default envs
    // no need to add builder and args because they are added later
    let mut envs = BTreeMap::new();
    envs.insert(NAME_KEY.to_string(), drv.name.to_string());
    envs.insert(OUTPUTS_KEY.to_string(), drv.outputs.join(" "));
    if let Some(ref fixed_hash) = drv.fixed_hash {
        envs.insert(FIXED_HASH_KEY.to_string(), fixed_hash.base64_with_algo());
    }
    envs.insert(SYSTEM_KEY.to_string(), drv.system.to_string());
    envs
}

fn drv_to_string<S>(drv: &StoreDrv) -> Result<String>
where
    S: Store,
{
    Ok(toml::to_string_pretty(&DrvSerializer {
        full_path: S::store_path,
        drv,
    })?)
}

static DRV_HASHES: LazyLock<Mutex<HashMap<StorePath, Hash>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

async fn hash_drv_path<S>(store: &S, p: &StorePath) -> Result<Hash>
where
    S: Store,
{
    {
        let hashes = DRV_HASHES.lock().unwrap();
        if let Some(hash) = hashes.get(p) {
            return Ok(hash.clone());
        }
    }
    let drv = Box::pin(store.read_drv(p)).await?;
    let hash = Box::pin(hash_drv(store, drv)).await?;
    {
        let mut hashes = DRV_HASHES.lock().unwrap();
        hashes.insert(p.clone(), hash.clone());
    }
    Ok(hash)
}

async fn hash_drv<S>(store: &S, drv: StoreDrv) -> Result<Hash>
where
    S: Store,
{
    let mut hasher = Sha512::new();
    let hash_str = if let Some(fixed_hash) = drv.fixed_hash {
        format!(
            "fixed:out:{}:{}",
            fixed_hash.base64_with_algo(),
            S::store_path(&drv.eq_classes["out"])
        )
    } else {
        let mut drv = drv.clone();
        let mut input_drvs = BTreeMap::new();
        for (p, outputs) in drv.input_drvs {
            let hash = Box::pin(hash_drv_path(store, &p)).await?;
            let path = make_path(&hash, p.name_part());
            input_drvs.insert(path, outputs);
        }
        drv.input_drvs = input_drvs;
        drv_to_string::<S>(&drv)?
    };
    hasher.update(hash_str);
    let hash = hasher.finalize();
    let hash = Hash::Sha512(Box::new(hash.into()));
    Ok(hash)
}

fn drv_to_buff<S>(drv: &StoreDrv) -> Result<BufReader<Cursor<String>>>
where
    S: Store,
{
    let drv_str = drv_to_string::<S>(drv)?;
    Ok(BufReader::new(Cursor::new(drv_str)))
}

#[derive(Default)]
struct BindRes {
    drvs: BTreeMap<StorePath, BTreeSet<Out>>,
    srcs: BTreeSet<StorePath>,
    res: Vec<String>,
}

async fn process_expr<S>(store: &S, expr: Expr) -> Result<BindRes>
where
    S: Store,
{
    process_expr_helper(store, expr, false).await
}

async fn process_expr_helper<S>(store: &S, expr: Expr, in_array: bool) -> Result<BindRes>
where
    S: Store,
{
    Ok(match expr {
        Expr::Str(s) => BindRes {
            // we debug print it because it might contain spaces
            res: vec![if in_array {
                format!("{:?}", s.as_ref())
            } else {
                s.to_string()
            }],
            ..Default::default()
        },
        Expr::Path(path) => {
            let p = store
                .add_to_store(
                    &path,
                    Opt {
                        algo: HashAlgo::Sha512,
                        refs: HashSet::new(),
                        eq_refs: None,
                        name: file_name(&path),
                        rewrites: HashMap::new(),
                        self_hash: None,
                    },
                )
                .await?;
            let outp = S::store_path(&p);
            BindRes {
                srcs: BTreeSet::from([p.clone()]),
                res: vec![outp],
                ..Default::default()
            }
        }
        Expr::Drv(drv_path) => {
            let drv = drv_path.drv;
            let out = drv_path.out.into_owned();
            let (d, p) = Box::pin(instantiate(store, &drv)).await?;
            let Some(eq_class) = d.eq_classes.get(&out) else {
                bail!("invalid output: {} not present", out);
            };
            let outp = if let Some(suff) = drv_path.suff {
                format!("{}{}", S::store_path(eq_class), suff)
            } else {
                S::store_path(eq_class)
            };
            BindRes {
                drvs: BTreeMap::from([(p, BTreeSet::from([out]))]),
                res: vec![outp],
                ..Default::default()
            }
        }
        Expr::Array(array) => {
            let mut drvs: BTreeMap<StorePath, BTreeSet<Out>> = BTreeMap::new();
            let mut srcs = BTreeSet::new();
            let mut res = Vec::new();
            for e in array.into_owned() {
                let ps = Box::pin(process_expr_helper(store, e, true)).await?;
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
