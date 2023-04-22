mod hash;
mod metadata_prefix;
pub use hash::hash_uri;
pub use metadata_prefix::{strip_metadata_prefix, write_metadata_prefix};
