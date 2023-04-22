use std::{io::ErrorKind, path::PathBuf};

use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use hyper::Uri;
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio_util::{compat::TokioAsyncWriteCompatExt, io::ReaderStream};

use super::base::{Cache, Cached, SetValue};

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
        let mut hasher = Sha256::new();

        // write input message
        hasher.update(key.to_string().as_bytes());

        // read hash digest and consume hasher
        let key_hash = hex::encode(hasher.finalize().as_slice());

        self.base_dir.join(key_hash)
    }
}

#[async_trait]
impl Cache for FsCache {
    #[tracing::instrument(skip(self))]
    async fn get(&self, key: &Uri) -> eyre::Result<Option<Cached>> {
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

        Ok(Some(Box::new(ReaderStream::new(file))))
    }

    #[tracing::instrument(skip(self, value))]
    async fn set(&self, key: &Uri, value: SetValue) -> eyre::Result<()> {
        let path = self.get_path(key);
        let file = File::create(path).await?;
        futures::io::copy(
            value.map(|part| Ok(part)).into_async_read(),
            &mut file.compat_write(),
        )
        .await?;

        Ok(())
    }
}
