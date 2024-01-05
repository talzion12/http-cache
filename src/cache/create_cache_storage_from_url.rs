use opendal::{
    services::{Fs, Gcs},
    Operator,
};

use url::Url;

use super::{Cache, OpendalStorage};

pub async fn create_cache_storage_from_url(url: &Url) -> color_eyre::Result<Box<dyn Cache>> {
    let result: Box<dyn Cache> = match url.scheme() {
        "file" => {
            let mut builder = Fs::default();
            builder.root(url.path());
            tracing::info!("Using filesystem cache at root {}", url.path());
            Box::new(OpendalStorage::new(Operator::new(builder)?.finish()))
        }
        "gs" => {
            let mut builder = Gcs::default();
            let bucket = url
                .host_str()
                .ok_or_else(|| color_eyre::eyre::eyre!("Must set url host as bucket"))?;
            let root = url.path();
            builder.bucket(bucket);
            builder.root(root);
            tracing::info!("Using google cloud cache at bucket {bucket} with root {root}");
            Box::new(OpendalStorage::new(Operator::new(builder)?.finish()))
        }
        other => color_eyre::eyre::bail!("Scheme not supported {other}"),
    };
    Ok(result)
}
