use anyhow::Result;
use oxide_instantiate::instantiate;
use oxide_pkgs::top_level::all_packages;
use oxide_store::stores::local;

#[tokio::main]
async fn main() -> Result<()> {
    let pkgs = all_packages::all_pkgs();
    let store = local::LocalStore::new().await?;
    instantiate(&store, &pkgs.hello).await?;
    instantiate(&store, &pkgs.perl).await?;
    Ok(())
}
