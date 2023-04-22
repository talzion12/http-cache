mod base;
mod fs;
mod redis;

pub use self::redis::RedisCache;
pub use base::Cache;
pub use fs::FsCache;
