use std::sync::Arc;

use async_trait::async_trait;
use redis::{AsyncCommands, aio::{ConnectionLike, ConnectionManager}};

use super::base::Cache;

pub struct RedisCache {
  redis: ConnectionManager,
  prefix: Option<Arc<str>>,
}

impl RedisCache {
  #[tracing::instrument()]
  pub async fn new(url: &str) -> eyre::Result<Self> {
    let client = redis::Client::open(url)?;
    let redis = ConnectionManager::new(client).await?;

    tracing::info!("Successfully connected to redis at {url}");
    
    Ok(Self {redis, prefix: None})
  }

  pub fn with_prefix(self, prefix: Arc<str>) -> Self {
    Self {
      prefix: Some(prefix.clone()),
      ..self
    }
  }

  async fn get_connection(&self) -> redis::RedisResult<impl ConnectionLike> {
    Ok(self.redis.clone())
  }

  fn get_key(&self, key: &str) -> String {
    match &self.prefix {
      Some(prefix) => format!("{prefix}:{key}"),
      None => key.to_string(),
    }
  }
}

#[async_trait]
impl Cache for RedisCache {
  #[tracing::instrument(skip(self))]
  async fn get(&self, key: &str) -> eyre::Result<Option<Vec<u8>>> {
    let mut redis = self.get_connection().await?;
    let res = redis.get(self.get_key(key)).await?;

    match &res {
      Some(_) => tracing::debug!("Found key: {key}"),
      None => tracing::debug!("Key not found: {key}"),
    };
    
    Ok(res)
  }

  #[tracing::instrument(skip(self, value))]
  async fn set(&self, key: &str, value: &[u8]) -> eyre::Result<()> {
    let mut redis = self.get_connection().await?;
    redis.set(self.get_key(key), value).await?;
    tracing::info!("Set key in redis");
    Ok(())
  }
}