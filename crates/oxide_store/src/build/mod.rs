mod builder;

use crate::{
    api::{EqRefs, Opt, Store},
    hash::utils::random_path,
    scan::scan,
    types::Realisation,
};
use anyhow::Result;
use builder::run_builder;
use oxide_core::{
    hash::HashAlgo,
    store::StorePath,
    types::{EqClass, Out},
};
use std::collections::{HashMap, HashSet};

pub async fn build<S>(store: &S, p: &StorePath) -> Result<HashMap<Out, StorePath>>
where
    S: Store,
{
    let mut drv = store.read_drv(&p).await?;
    let mut inputs = HashSet::new();
    for (path, _) in drv.input_drvs.iter() {
        build(store, &path).await?;

        let input_drv = store.read_drv(&path).await?;
        for (out, eq_class) in input_drv.eq_classes {
            let trusted_paths = store.trusted_paths(&eq_class, &out).await?;
            for tp in trusted_paths {
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
    let inputs = resolve(store, inputs).await?;

    for r in inputs.iter() {
        // TODO: replace this with the faster search algorithm used in oxide_store/hash/scan.rs
        let e = r.eq_class.hash_part();
        let p = r.path.hash_part();
        drv.builder = drv.builder.replace(e, p);
        drv.args = drv.args.into_iter().map(|arg| arg.replace(e, p)).collect();
        drv.envs = drv
            .envs
            .into_iter()
            .map(|(k, v)| (k, v.replace(e, p)))
            .collect();
    }

    drv.eq_classes = drv
        .eq_classes
        .into_iter()
        .map(|(out, eq_class)| (out, random_path(eq_class.name_part())))
        .collect();
    drv.envs.extend(
        drv.eq_classes
            .iter()
            .map(|(out, eq_class)| (out.to_string(), eq_class.to_string())),
    );

    run_builder::<S>(&drv).await?;

    let refs = inputs.iter().map(|r| r.path.clone()).collect();
    let mut outputs = HashMap::new();
    for (out, eq_class) in drv.eq_classes {
        let tmp_path = S::store_path(&eq_class);
        let refs = scan(&tmp_path, &refs).await?;
        let name = eq_class.name_part().to_string();
        let self_hash = Some(eq_class.to_hash_part());
        let output = store
            .add_to_store(
                &tmp_path,
                Opt {
                    algo: HashAlgo::Sha512,
                    refs,
                    eq_refs: Some(EqRefs {
                        eq_class,
                        out: out.clone(),
                        refs: inputs.clone(),
                    }),
                    name,
                    rewrites: HashMap::new(),
                    self_hash,
                },
            )
            .await?;
        outputs.insert(out, output);
    }
    Ok(outputs)
}

fn selected_paths(
    conflicts: HashMap<(EqClass, Out), Vec<StorePath>>,
) -> HashMap<(EqClass, Out), StorePath> {
    let mut selected = HashMap::new();
    for ((eq_class, out), conflict) in conflicts {
        if conflict.len() > 1 {
            panic!("right now conflicts cannot be resolved");
        }
        let path = conflict.into_iter().next().unwrap();
        selected.insert((eq_class, out), path);
    }
    selected
}

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
    return Ok(r);
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
    let refs = store.realisation_refs(&r).await?;
    res.insert(r);
    for r in refs {
        Box::pin(closure_helper(store, r, res)).await?;
    }
    Ok(())
}
