use async_trait::async_trait;
use futures::{io::BufReader, StreamExt};
use http::Uri;
use opendal::{ErrorKind, Operator};
use tokio_util::{compat::FuturesAsyncReadCompatExt, io::ReaderStream};

use crate::cache::metadata::CacheMetadata;

use super::{
    base::{GetReturn, SetBody},
    util::{hash_uri, strip_metadata_prefix, write_metadata_prefix},
    Cache,
};

pub struct OpendalStorage {
    operator: Operator,
}

impl OpendalStorage {
    pub fn new(operator: Operator) -> Self {
        Self { operator }
    }
}

#[async_trait]
impl Cache for OpendalStorage {
    async fn get(&self, key: &Uri) -> eyre::Result<GetReturn> {
        let reader = match self.operator.reader(&hash_uri(key)).await {
            Ok(file) => file,
            Err(error) => {
                if error.kind() == ErrorKind::NotFound {
                    return Ok(None);
                } else {
                    return Err(error.into());
                }
            }
        };

        let mut reader = BufReader::new(reader);

        let metadata = strip_metadata_prefix(&mut reader).await?;

        Ok(Some((
            metadata,
            Box::new(ReaderStream::new(reader.compat())),
        )))
    }
    async fn set(
        &self,
        key: &Uri,
        mut value: SetBody,
        metadata: CacheMetadata,
    ) -> eyre::Result<()> {
        let mut writer = self.operator.writer(&hash_uri(key)).await?;

        write_metadata_prefix(&mut writer, &metadata).await?;

        while let Some(part) = value.next().await {
            writer.append(part).await?;
        }

        Ok(())
    }
}
