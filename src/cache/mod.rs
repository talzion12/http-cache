mod service;
mod redis;
mod base;

pub use service::{CachingLayer, CachingService};
pub use self::redis::RedisCache;
pub use base::Cache;