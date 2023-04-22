use std::{convert::Infallible, sync::Arc};

use async_trait::async_trait;
use futures::stream::StreamExt;
use hyper::{body::Bytes, Uri};
use redis::{
    aio::{ConnectionLike, ConnectionManager},
    AsyncCommands,
};

use super::base::{Cache, Cached, SetValue};

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

        Ok(Self {
            redis,
            prefix: None,
        })
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

    fn get_key(&self, key: &Uri) -> String {
        match &self.prefix {
            Some(prefix) => format!("{prefix}:{key}"),
            None => key.to_string(),
        }
    }
}

#[async_trait]
impl Cache for RedisCache {
    #[tracing::instrument(skip(self))]
    async fn get(&self, key: &Uri) -> eyre::Result<Option<Cached>> {
        let mut redis = self.get_connection().await?;
        let res: Option<Vec<u8>> = redis.get(self.get_key(key)).await?;

        Ok(match res {
            Some(cached) => {
                tracing::debug!("Found key: {key}");
                Some(Box::new(futures::stream::once(futures::future::ready(Ok(
                    Bytes::from(cached),
                )))))
            }
            None => {
                tracing::debug!("Key not found: {key}");
                None
            }
        })
    }

    #[tracing::instrument(skip(self, value))]
    async fn set(&self, key: &Uri, value: SetValue) -> eyre::Result<()> {
        todo!()
        // let mut b: Vec<u8> = Vec::new();
        // while let Some(v) = value.next().await {
        //     if let Ok(v) = v {
        //         b.extend_from_slice(v.as_bytes);
        //     } else {
        //     }
        // }
        // let mut redis = self.get_connection().await?;
        // redis.set(self.get_key(key), value).await?;
        // tracing::info!("Set key in redis");
        // Ok(())
    }
}
