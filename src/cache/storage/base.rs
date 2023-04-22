use std::ops::Deref;

use async_trait::async_trait;
use futures::Stream;
use hyper::{body::Bytes, Uri};

pub type Cached = Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Unpin + Send>;
pub type SetValue = futures::channel::mpsc::Receiver<Bytes>;

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &Uri) -> eyre::Result<Option<Cached>>;
    async fn set(&self, key: &Uri, value: SetValue) -> eyre::Result<()>;
}

#[async_trait]
impl Cache for Box<dyn Cache> {
    async fn get(&self, key: &Uri) -> eyre::Result<Option<Cached>> {
        self.deref().get(key).await
    }
    async fn set(&self, key: &Uri, value: SetValue) -> eyre::Result<()> {
        self.deref().set(key, value).await
    }
}
