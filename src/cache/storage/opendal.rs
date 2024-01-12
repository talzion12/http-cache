use futures::{io::BufReader, StreamExt};
use http::Uri;
use opendal::Operator;
use tokio_util::{compat::FuturesAsyncReadCompatExt, io::ReaderStream};

use crate::cache::metadata::CacheMetadata;

use super::{
    base::{GetReturn, SetBody},
    util::{hash_uri, strip_metadata_prefix, write_metadata_prefix},
    CacheStorage,
};

pub struct OpendalStorage {
    operator: Operator,
    buffer_size: Option<usize>,
}

impl OpendalStorage {
    pub fn new(operator: Operator) -> Self {
        Self {
            operator,
            buffer_size: None,
        }
    }
}

impl OpendalStorage {
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = Some(buffer_size);
        self
    }
}

#[async_trait::async_trait]
impl CacheStorage for OpendalStorage {
    async fn get(&self, uri: &Uri, prefix: Option<&str>) -> eyre::Result<GetReturn> {
        let reader = match self.operator.reader(&get_cache_key(uri, prefix)).await {
            Ok(file) => file,
            Err(error) => {
                if error.kind() == opendal::ErrorKind::NotFound {
                    return Ok(None);
                } else {
                    return Err(error.into());
                }
            }
        };

        let mut reader = BufReader::new(reader);

        let metadata = match strip_metadata_prefix(&mut reader).await {
            Ok(metadata) => metadata,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::NotFound {
                    return Ok(None);
                } else {
                    return Err(error.into());
                }
            }
        };

        Ok(Some((
            metadata,
            Box::new(ReaderStream::new(reader.compat())),
        )))
    }
    async fn set(
        &self,
        uri: &Uri,
        mut value: SetBody,
        metadata: &CacheMetadata,
        prefix: Option<&str>,
    ) -> eyre::Result<()> {
        let mut writer_builder = self.operator.writer_with(&get_cache_key(uri, prefix));

        if let Some(buffer_size) = self.buffer_size {
            writer_builder = writer_builder.buffer(buffer_size)
        }

        let mut writer = writer_builder.await?;

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
