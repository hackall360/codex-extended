use std::sync::Arc;

use clap::Parser;
use codex_common::CliConfigOverrides;
use codex_core::ConversationManager;
use codex_core::NewConversation;
use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_core::protocol::{Event, EventMsg, Submission};
use codex_login::AuthManager;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tracing::{error, info};

/// Run Codex as a TCP server using the JSON protocol.
#[derive(Debug, Parser)]
pub struct ServerCli {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    /// Address to bind the server to
    #[clap(long, default_value = "127.0.0.1:3030")]
    bind: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = ServerCli::parse();
    run_main(cli).await
}

async fn run_main(opts: ServerCli) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let overrides_vec = opts
        .config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = Config::load_with_cli_overrides(overrides_vec, ConfigOverrides::default())?;
    let auth = AuthManager::shared(config.codex_home.clone(), config.preferred_auth_method);
    let conversation_manager = Arc::new(ConversationManager::new(auth));

    let listener = TcpListener::bind(&opts.bind).await?;
    info!("listening on {}", &opts.bind);

    loop {
        let (stream, addr) = listener.accept().await?;
        let cm = conversation_manager.clone();
        let cfg = config.clone();
        tokio::spawn(async move {
            info!("client connected from {addr}");
            if let Err(e) = handle_connection(stream, cm, cfg).await {
                error!("connection error: {e:#}");
            }
            info!("client {addr} disconnected");
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    conversation_manager: Arc<ConversationManager>,
    config: Config,
) -> anyhow::Result<()> {
    let NewConversation {
        conversation,
        session_configured,
        ..
    } = conversation_manager.new_conversation(config).await?;

    let synthetic = Event {
        id: String::new(),
        msg: EventMsg::SessionConfigured(session_configured),
    };

    let (read_half, mut write_half) = stream.into_split();
    let line = serde_json::to_string(&synthetic)?;
    write_half.write_all(line.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    let reader = BufReader::new(read_half);

    let sq = {
        let conversation = conversation.clone();
        async move {
            let mut lines = reader.lines();
            loop {
                let line = tokio::select! {
                    _ = signal::ctrl_c() => { break; },
                    res = lines.next_line() => res,
                };

                match line {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<Submission>(&line) {
                            Ok(sub) => {
                                if let Err(e) = conversation.submit_with_id(sub).await {
                                    error!("{e:#}");
                                    break;
                                }
                            }
                            Err(e) => error!("invalid submission: {e}"),
                        }
                    }
                    _ => break,
                }
            }
        }
    };

    let eq = async move {
        loop {
            let event = tokio::select! {
                _ = signal::ctrl_c() => break,
                ev = conversation.next_event() => ev,
            };
            match event {
                Ok(event) => {
                    if let Ok(s) = serde_json::to_string(&event) {
                        if write_half.write_all(s.as_bytes()).await.is_err() {
                            break;
                        }
                        if write_half.write_all(b"\n").await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("{e:#}");
                    break;
                }
            }
        }
    };

    tokio::join!(sq, eq);
    Ok(())
}
