use eyre::Context;
use opendal::{
    services::{Fs, Gcs},
    Operator,
};

use url::Url;

use super::{CacheStorage, OpendalStorage};

pub fn create_cache_storage_from_url(url: &Url) -> color_eyre::Result<Box<dyn CacheStorage>> {
    let result: Box<dyn CacheStorage> = match url.scheme() {
        "file" => {
            let mut builder = Fs::default();
            let path = url.path();
            builder.root(path);
            tracing::info!("Using filesystem cache at root {}", path);
            Box::new(OpendalStorage::new(Operator::new(builder)?.finish()))
        }
        "gs" => {
            let mut builder = Gcs::default();
            let bucket = url
                .host_str()
                .ok_or_else(|| color_eyre::eyre::eyre!("Must set url host as bucket"))?;
            let root = url.path();

            // The buffer size should be a multiple of 256 KiB (256 x 1024 bytes), unless it's the last chunk that completes the upload.
            // Larger chunk sizes typically make uploads faster, but note that there's a tradeoff between speed and memory usage.
            // It's recommended that you use at least 8 MiB for the chunk size.
            //
            // Reference: [Perform resumable uploads](https://cloud.google.com/storage/docs/performing-resumable-uploads)
            let mut buffer_size: Option<usize> = None;

            for (key, value) in url.query_pairs() {
                if key == "buffer_size" {
                    buffer_size = Some(value.parse().context("Failed to parse buffer size")?);
                }
            }

            builder.bucket(bucket);
            builder.root(root);
            tracing::info!("Using google cloud cache at bucket {bucket} with root {root}");
            Box::new(
                OpendalStorage::new(Operator::new(builder)?.finish())
                    .with_buffer_size(buffer_size.unwrap_or_else(|| 1024 * 1024)),
            )
        }
        other => color_eyre::eyre::bail!("Scheme not supported {other}"),
    };
    Ok(result)
}
