use async_trait::async_trait;

#[async_trait]
pub trait Cache {
    async fn get(&self, key: &str) -> eyre::Result<Option<String>>;
    async fn set(&self, key: &str, value: String) -> eyre::Result<()>;
}
