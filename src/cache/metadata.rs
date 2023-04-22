use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct CacheMetadata {
    pub status: u16,
    pub headers: HashMap<String, Vec<u8>>,
}
