use futures::{io::AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::cache::metadata::CacheMetadata;

pub async fn strip_metadata_prefix<R>(reader: &mut R) -> Result<CacheMetadata, std::io::Error>
where
    R: AsyncBufRead + Unpin,
{
    let mut metadata_buffer = Vec::new();
    reader.read_until(b'\n', &mut metadata_buffer).await?;

    let metadata: CacheMetadata = serde_json::from_slice(&metadata_buffer)?;

    Ok(metadata)
}

pub async fn write_metadata_prefix<W>(
    writer: &mut W,
    metadata: &CacheMetadata,
) -> Result<(), std::io::Error>
where
    W: AsyncWrite + Unpin,
{
    writer
        .write_all(serde_json::to_string(&metadata)?.as_bytes())
        .await?;
    writer.write_all("\n".as_bytes()).await?;

    Ok(())
}
