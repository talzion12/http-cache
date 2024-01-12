use std::{collections::HashMap, str::FromStr};

use eyre::ContextCompat;
use futures::SinkExt;
use http::Uri;
use hyper::body::{Body, Bytes};
use url::Url;

use crate::{
    cache::{create_cache_storage_from_url, metadata::CacheMetadata, CacheStorage},
    test::fixture::{dotenv_fixture, tracing_fixture},
};

/// This test uses the `http-cache-test` bucket to check that the
/// Google cloud storage opendal backend actually works.
///
/// To run this test it's required to set the `GOOGLE_APPLICATION_CREDENTIALS` env variable
/// in the `.env` file. The service account needs to have the following roles:
/// * Storage Legacy Object Reader
/// * Storage Object Creator
#[tokio::test]
#[ignore]
async fn test_gcs_storage() -> eyre::Result<()> {
    tracing_fixture();
    dotenv_fixture()?;

    let bucket = "http-cache-test";
    let root = format!(
        "root-{}",
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)?
    );

    let storage = create_cache_storage_from_url(&Url::from_str(&format!("gs://{bucket}/{root}"))?)?;

    test_storage(storage.as_ref()).await?;

    Ok(())
}

#[tokio::test]
async fn test_fs_storage() -> eyre::Result<()> {
    tracing_fixture();
    dotenv_fixture()?;

    let temp_dir = tempfile::TempDir::new()?;
    let root = temp_dir
        .path()
        .to_str()
        .context("Path is not a valid string")?;

    let storage = create_cache_storage_from_url(&Url::from_str(&format!("file:{root}"))?)?;

    test_storage(storage.as_ref()).await?;

    Ok(())
}

async fn test_storage(storage: &dyn CacheStorage) -> eyre::Result<()> {
    let uri = Uri::try_from("https://www.google.com")?;

    assert!(
        storage.get(&uri, None).await?.is_none(),
        "get should return None if not existent"
    );

    let (mut sender, receiver) = futures::channel::mpsc::channel(1);

    let content = b"Hello world";
    let metadata = CacheMetadata {
        status: 200,
        headers: HashMap::new(),
    };

    sender.send(Bytes::from_static(content)).await?;
    sender.disconnect();

    storage.set(&uri, receiver, &metadata, None).await?;

    let (metadata_from_storage, stream) = storage
        .get(&uri, None)
        .await?
        .context("Expected to find item")?;

    assert_eq!(
        hyper::body::to_bytes(Body::wrap_stream(stream))
            .await?
            .as_ref(),
        content
    );

    assert_eq!(metadata_from_storage.status, metadata.status);
    assert_eq!(metadata_from_storage.headers, metadata.headers);

    Ok(())
}
