use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use codex_common::CliConfigOverrides;
use codex_core::ConversationManager;
use codex_core::NewConversation;
use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_core::protocol::Event;
use codex_core::protocol::EventMsg;
use codex_core::protocol::Submission;
use codex_login::AuthManager;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tracing::error;
use tracing::info;

#[derive(Debug, Parser)]
struct ServerCli {
    #[arg(long, default_value = "127.0.0.1:7878")]
    listen: SocketAddr,

    #[clap(flatten)]
    config_overrides: CliConfigOverrides,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = ServerCli::parse();
    let overrides_vec = cli
        .config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = Config::load_with_cli_overrides(overrides_vec, ConfigOverrides::default())?;

    let listener = TcpListener::bind(cli.listen)
        .await
        .context("binding listen address")?;
    let auth_manager = AuthManager::shared(config.codex_home.clone(), config.preferred_auth_method);
    let manager = Arc::new(ConversationManager::new(auth_manager));

    loop {
        let (stream, addr) = listener.accept().await?;
        info!(?addr, "accepted connection");
        let manager = Arc::clone(&manager);
        let config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, manager, config).await {
                error!("connection error: {e:#}");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    manager: Arc<ConversationManager>,
    config: Config,
) -> anyhow::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half).lines();

    let NewConversation {
        conversation,
        session_configured,
        ..
    } = manager.new_conversation(config).await?;

    let init_event = Event {
        id: String::new(),
        msg: EventMsg::SessionConfigured(session_configured),
    };
    let init_str = serde_json::to_string(&init_event)?;
    write_half.write_all(init_str.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    write_half.flush().await?;

    let conversation_read = conversation.clone();
    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = reader.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<Submission>(&line) {
                Ok(sub) => {
                    if let Err(e) = conversation_read.submit_with_id(sub).await {
                        error!("{e:#}");
                        break;
                    }
                }
                Err(e) => error!("invalid submission: {e}"),
            }
        }
    });

    let mut writer = write_half;
    let event_task = tokio::spawn(async move {
        loop {
            match conversation.next_event().await {
                Ok(ev) => match serde_json::to_string(&ev) {
                    Ok(s) => {
                        if writer.write_all(s.as_bytes()).await.is_err() {
                            break;
                        }
                        if writer.write_all(b"\n").await.is_err() {
                            break;
                        }
                        if writer.flush().await.is_err() {
                            break;
                        }
                    }
                    Err(e) => error!("serialize event: {e}"),
                },
                Err(e) => {
                    error!("{e:#}");
                    break;
                }
            }
        }
    });

    let _ = tokio::join!(read_task, event_task);
    Ok(())
}
