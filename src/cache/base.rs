use async_trait::async_trait;

#[async_trait]
pub trait Cache {
    async fn get(&self, key: &str) -> eyre::Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> eyre::Result<()>;
}
