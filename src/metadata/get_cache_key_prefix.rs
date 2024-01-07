use std::sync::Arc;

use http::HeaderValue;

pub const CACHE_KEY_PREFIX_HEADER_KEY: &'static str = "x-http-cache-key-prefix";

pub fn get_cache_key_prefix(cache_key_header_value: Option<HeaderValue>) -> Option<Arc<str>> {
    let res = cache_key_header_value?.to_str().ok()?.into();
    Some(res)
}
