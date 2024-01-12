pub fn dotenv_fixture() -> eyre::Result<()> {
    use eyre::Context;

    match dotenv::dotenv() {
        Ok(path) => {
            tracing::info!("Loaded env from {path:?}")
        }
        Err(dotenv::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("Not loading .env because it wasn't found");
        }
        Err(error) => return Err(error).context("Failed to load .env"),
    };

    Ok(())
}
