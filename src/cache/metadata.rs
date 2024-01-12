use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CacheMetadata {
    pub status: u16,
    pub headers: HashMap<String, Vec<u8>>,
}
