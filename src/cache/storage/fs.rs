use std::{io::ErrorKind, path::PathBuf};

use async_trait::async_trait;
use futures::{
    io::{BufReader, BufWriter},
    StreamExt, TryStreamExt,
};
use hyper::Uri;
use tokio::fs::File;
use tokio_util::{
    compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt},
    io::ReaderStream,
};

use crate::cache::metadata::CacheMetadata;

use super::{
    base::{GetReturn, SetBody},
    util::{hash_uri, strip_metadata_prefix, write_metadata_prefix},
    Cache,
};

pub struct FsCache {
    base_dir: PathBuf,
}

impl FsCache {
    #[tracing::instrument()]
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

impl FsCache {
    fn get_path(&self, key: &Uri) -> PathBuf {
        let key_hash = hash_uri(key);
        self.base_dir.join(key_hash)
    }
}

#[async_trait]
impl Cache for FsCache {
    #[tracing::instrument(skip(self))]
    async fn get(&self, key: &Uri) -> eyre::Result<GetReturn> {
        let path = self.get_path(key);
        tracing::debug!("Path is {path:?}");
        let file = match File::open(path).await {
            Ok(file) => file,
            Err(error) => {
                if error.kind() == ErrorKind::NotFound {
                    return Ok(None);
                } else {
                    return Err(error.into());
                }
            }
        };

        let mut file = BufReader::new(file.compat());
        let metadata = strip_metadata_prefix(&mut file).await?;

        Ok(Some((metadata, Box::new(ReaderStream::new(file.compat())))))
    }

    #[tracing::instrument(skip(self, value, metadata))]
    async fn set(&self, key: &Uri, value: SetBody, metadata: CacheMetadata) -> eyre::Result<()> {
        let path = self.get_path(key);
        let mut file = BufWriter::new(File::create(path).await?.compat_write());

        write_metadata_prefix(&mut file, &metadata).await?;

        futures::io::copy(value.map(|part| Ok(part)).into_async_read(), &mut file).await?;

        Ok(())
    }
}
