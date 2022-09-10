use async_trait::async_trait;
use eyre::Context;
use redis::Commands;

use super::base::Cache;

pub struct RedisCache {
  redis: redis::Client,
}

impl RedisCache {
  #[tracing::instrument()]
  pub fn new(redis: redis::Client) -> eyre::Result<Self> {
    redis.get_connection().wrap_err("Failed to connect to redis")?;
    tracing::info!("Successfully connected to redis at {}", redis.get_connection_info().addr);
    Ok(Self {redis})
  }
}

#[async_trait]
impl Cache for RedisCache {
  #[tracing::instrument(skip(self))]
  async fn get(&self, key: &str) -> eyre::Result<Option<String>> {
    let mut redis = self.redis.get_connection()?;
    let res = redis.get(key)?;

    match &res {
      Some(value) => tracing::debug!("Found key: {key}"),
      None => tracing::debug!("Key not found: {key}"),
    };
    
    Ok(res)
  }

  #[tracing::instrument(skip(self))]
  async fn set(&self, key: &str, value: String) -> eyre::Result<()> {
    Ok(())
  }
}