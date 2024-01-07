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

#[async_trait::async_trait]
impl Cache for OpendalStorage {
    async fn get(&self, uri: &Uri, prefix: Option<&str>) -> eyre::Result<GetReturn> {
        let reader = match self.operator.reader(&get_cache_key(uri, prefix)).await {
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
        uri: &Uri,
        mut value: SetBody,
        metadata: CacheMetadata,
        prefix: Option<&str>,
    ) -> eyre::Result<()> {
        let mut writer = self.operator.writer(&get_cache_key(uri, prefix)).await?;

        write_metadata_prefix(&mut writer, &metadata).await?;

        while let Some(part) = value.next().await {
            writer.write(part).await?;
        }

        writer.close().await?;

        Ok(())
    }
}

fn get_cache_key(uri: &Uri, prefix: Option<&str>) -> String {
    let hash = hash_uri(uri);
    if let Some(prefix) = prefix {
        format!("{prefix}{hash}")
    } else {
        hash
    }
}
