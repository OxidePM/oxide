mod builder;

use crate::{
    api::{EqRefs, Opt, Store},
    hash::{rewrite_str, scan_for_refs, utils::random_path},
    types::Realisation,
};
use anyhow::{bail, Result};
use builder::run_builder;
use log::info;
use oxide_core::{
    drv::StoreDrv,
    hash::HashAlgo,
    store::StorePath,
    types::{EqClass, Out},
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub async fn build<S>(store: &S, p: &StorePath) -> Result<HashMap<Out, StorePath>>
where
    S: Store,
{
    let mut drv = store.read_drv(p).await?;

    'b: {
        // if all the eq_classes have a trusted path do not build again
        let mut outs = HashMap::new();
        for (out, eq_class) in &drv.eq_classes {
            let trusted = store.trusted_paths(eq_class, out).await?;
            if let Some(path) = trusted.into_iter().next() {
                outs.insert(out.clone(), path.clone());
            } else {
                break 'b;
            }
        }
        info!("building {p}: trusted path found");
        return Ok(outs);
    }

    for path in drv.input_drvs.keys() {
        Box::pin(build(store, path)).await?;
    }

    let inputs = inputs(store, &drv).await?;
    info!("building: {p}");

    let mut mappings = HashMap::new();
    for r in &inputs {
        // how can we avoid this clone?
        mappings.insert(r.eq_class.clone(), r.path.clone());
    }
    rewrite_str(&mut drv.builder, &mappings);
    for arg in &mut drv.args {
        rewrite_str(arg, &mappings);
    }
    for v in drv.envs.values_mut() {
        rewrite_str(v, &mappings);
    }

    let outputs = drv
        .eq_classes
        .iter()
        .map(|(out, eq_class)| (out.clone(), random_path(eq_class.name_part())))
        .collect::<HashMap<_, _>>();
    drv.envs.extend(
        outputs
            .iter()
            .map(|(out, eq_class)| (out.clone(), S::store_path(eq_class))),
    );

    run_builder::<S>(&drv).await?;
    if drv.builtin().is_some() {
        return Ok(HashMap::new());
    }

    // check that every output path was produced
    for (out, eq_class) in &outputs {
        let tmp_path = S::store_path(eq_class);
        let tmp_path = Path::new(&tmp_path);
        if !tmp_path.exists() {
            bail!("builder failed to produce output {}", out);
        }
    }

    let mut refs = inputs
        .iter()
        .map(|r| r.path.clone())
        .collect::<HashSet<_>>();
    refs.extend(drv.input_srcs);
    let mut outs = HashMap::new();
    for (out, eq_class) in drv.eq_classes {
        let self_hash = outputs[&out].clone();
        let tmp_path = S::store_path(&self_hash);

        let mut refs = refs.clone();
        refs.insert(self_hash.clone());
        let refs = scan_for_refs(&tmp_path, refs).await?;

        // TODO: what about self ref
        let eq_refs = inputs
            .iter()
            .filter(|&i| refs.contains(&i.path))
            .cloned()
            .collect();

        let name = eq_class.name_part().to_string();
        let self_hash = Some(self_hash);
        let output = store
            .add_to_store(
                &tmp_path,
                Opt {
                    algo: HashAlgo::Sha512,
                    refs,
                    eq_refs: Some(EqRefs {
                        eq_class,
                        out: out.clone(),
                        refs: eq_refs,
                    }),
                    name,
                    rewrites: HashMap::new(),
                    self_hash,
                },
            )
            .await?;
        outs.insert(out, output);
    }
    Ok(outs)
}

pub async fn inputs<S>(store: &S, drv: &StoreDrv) -> Result<Vec<Realisation>>
where
    S: Store,
{
    let mut inputs = HashSet::new();
    for path in drv.input_drvs.keys() {
        let input_drv = store.read_drv(path).await?;
        for (out, eq_class) in input_drv.eq_classes {
            let trusted = store.trusted_paths(&eq_class, &out).await?;
            for tp in trusted {
                let closure = closure(
                    store,
                    Realisation {
                        eq_class: eq_class.clone(),
                        out: out.clone(),
                        path: tp,
                    },
                )
                .await?;
                inputs.extend(closure);
            }
        }
    }
    resolve(store, inputs).await
}

fn selected_paths(
    conflicts: HashMap<(EqClass, Out), Vec<StorePath>>,
) -> HashMap<(EqClass, Out), StorePath> {
    let mut selected = HashMap::new();
    for ((eq_class, out), conflict) in conflicts {
        assert!(
            conflict.len() == 1,
            "right now conflicts cannot be resolved"
        );
        let path = conflict.into_iter().next().unwrap();
        selected.insert((eq_class, out), path);
    }
    selected
}

#[allow(clippy::pedantic)]
async fn maybe_rewrite<S>(
    store: &S,
    r: Realisation,
    selected: &HashMap<(EqClass, Out), StorePath>,
) -> Result<Realisation>
where
    S: Store,
{
    _ = store;
    _ = selected;
    // TODO: rewrite
    Ok(r)
}

async fn resolve<S>(store: &S, inputs: HashSet<Realisation>) -> Result<Vec<Realisation>>
where
    S: Store,
{
    let mut conflicts: HashMap<(EqClass, Out), Vec<StorePath>> = HashMap::new();
    for r in inputs {
        conflicts
            .entry((r.eq_class, r.out))
            .or_default()
            .push(r.path);
    }
    let selected = selected_paths(conflicts);
    let mut realisations = Vec::new();
    for ((eq_class, out), path) in selected.clone() {
        let r = Realisation {
            eq_class,
            out,
            path,
        };
        let rewrite = maybe_rewrite(store, r, &selected).await?;
        realisations.push(rewrite);
    }
    Ok(realisations)
}

async fn closure<S>(store: &S, r: Realisation) -> Result<HashSet<Realisation>>
where
    S: Store,
{
    let mut res = HashSet::new();
    closure_helper(store, r, &mut res).await?;
    Ok(res)
}

async fn closure_helper<S>(store: &S, r: Realisation, res: &mut HashSet<Realisation>) -> Result<()>
where
    S: Store,
{
    let closure = store.realisation_refs(&r).await?;
    res.insert(r);
    for r in closure {
        Box::pin(closure_helper(store, r, res)).await?;
    }
    Ok(())
}
