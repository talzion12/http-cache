use std::path::PathBuf;

use url::Url;

use super::{Cache, FsCache};

pub async fn create_cache_storage_from_url(url: &Url) -> color_eyre::Result<Box<dyn Cache>> {
    let result: Box<dyn Cache> = match url.scheme() {
        "file" => Box::new(FsCache::new(PathBuf::from(url.path()))),
        other => color_eyre::eyre::bail!("Scheme not supported {other}"),
    };
    Ok(result)
}
