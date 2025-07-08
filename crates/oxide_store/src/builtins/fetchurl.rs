use super::Ctx;
use anyhow::{bail, Result};
use futures_util::TryStreamExt;
use oxide_core::utils::{EXEC_FILE_PERMISSION, FILE_PERMISSION};
use tokio::{
    fs::OpenOptions,
    io::{AsyncWriteExt as _, BufWriter},
};

pub async fn fetch_url(ctx: Ctx<'_>) -> Result<()> {
    if ctx.drv.fixed_hash.is_none() {
        bail!(r#""builtin:fetchurl" must be a fixed output derivation"#);
    }
    let Some(out) = ctx.outputs.get("out") else {
        bail!(r#""builtin:fetchurl" requires an 'out' output"#);
    };

    let store_path = out;
    let Some(main_url) = ctx.drv.envs.get("url") else {
        bail!(r#""builtin:fetchurl" must have a url"#);
    };
    let unpack = ctx.drv.envs.get("unpack").is_some_and(|v| v == "1");
    if unpack {
        todo!()
    }
    let executable = ctx.drv.envs.get("executable").is_some_and(|v| v == "1");
    let client = reqwest::Client::new();
    let response = client.get(main_url).send().await?;

    if !response.status().is_success() {
        bail!(
            r#""builtin:fetchurl" failed to download url {} with code {}"#,
            main_url,
            response.status()
        );
    }

    let mode = if executable {
        EXEC_FILE_PERMISSION
    } else {
        FILE_PERMISSION
    };
    let file = OpenOptions::new()
        .mode(mode)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&store_path)
        .await?;
    let mut writer = BufWriter::new(file);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.try_next().await? {
        writer.write_all(&chunk).await?;
    }

    Ok(())
}
