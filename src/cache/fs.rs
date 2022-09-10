use async_trait::async_trait;
use redis::{AsyncCommands, aio::ConnectionManager};

use super::base::Cache;

pub struct FsCache {
  fs: ConnectionManager,
}

impl RedisCache {
  #[tracing::instrument()]
  pub async fn new(url: String) -> eyre::Result<Self> {
    let client = redis::Client::open(url.as_str())?;
    let redis = ConnectionManager::new(client).await?;

    tracing::info!("Successfully connected to redis at {url}");
    
    Ok(Self {redis})
  }
}

#[async_trait]
impl Cache for RedisCache {
  #[tracing::instrument(skip(self))]
  async fn get(&self, key: &str) -> eyre::Result<Option<String>> {
    let res = self.redis.get(key).await?;

    match &res {
      Some(value) => tracing::debug!("Found key: {key}"),
      None => tracing::debug!("Key not found: {key}"),
    };
    
    Ok(res)
  }

  #[tracing::instrument(skip(self, value))]
  async fn set(&self, key: &str, value: String) -> eyre::Result<()> {
    self.redis.set(key, value).await?;
    tracing::info!("Set key in redis");
    Ok(())
  }
}