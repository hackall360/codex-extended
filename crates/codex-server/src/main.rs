use codex_core::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::load()?;
    let (_addr, handle) = codex_server::start(cfg).await?;
    handle.await?;
    Ok(())
}
