mod base;
mod opendal;
#[cfg(test)]
mod test;
mod util;

pub use self::opendal::OpendalStorage;
pub use base::{CacheStorage, GetBody};
