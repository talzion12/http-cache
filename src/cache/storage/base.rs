use super::super::metadata::CacheMetadata;
use futures::Stream;
use hyper::{body::Bytes, Uri};

pub type GetBody = Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Unpin + Send>;
pub type GetReturn = Option<(CacheMetadata, GetBody)>;
pub type SetBody = futures::channel::mpsc::Receiver<Bytes>;

#[async_trait::async_trait]
pub trait CacheStorage: Send + Sync + 'static {
    async fn get(&self, key: &Uri, prefix: Option<&str>) -> eyre::Result<GetReturn>;
    async fn set(
        &self,
        key: &Uri,
        value: SetBody,
        metadata: &CacheMetadata,
        prefix: Option<&str>,
    ) -> eyre::Result<()>;
}
