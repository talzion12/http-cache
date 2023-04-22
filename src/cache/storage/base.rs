use super::super::metadata::CacheMetadata;
use async_trait::async_trait;
use futures::Stream;
use hyper::{body::Bytes, Uri};

pub type GetBody = Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Unpin + Send>;
pub type GetReturn = Option<(CacheMetadata, GetBody)>;
pub type SetBody = futures::channel::mpsc::Receiver<Bytes>;

#[async_trait]
pub trait Cache: Send + Sync + 'static {
    async fn get(&self, key: &Uri) -> eyre::Result<GetReturn>;
    async fn set(&self, key: &Uri, value: SetBody, metadata: CacheMetadata) -> eyre::Result<()>;
}
