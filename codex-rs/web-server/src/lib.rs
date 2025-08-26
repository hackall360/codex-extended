use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::routing::get;
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
use futures::SinkExt;
use futures::StreamExt;
use tokio::net::TcpListener;
use tracing::error;
use tracing::info;

#[derive(Debug, Parser)]
pub struct WebCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// Address to bind the WebSocket server to
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub addr: String,
}

pub async fn run_main(opts: WebCli) -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let WebCli {
        config_overrides,
        addr,
    } = opts;
    let overrides_vec = config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;

    let config = Config::load_with_cli_overrides(overrides_vec, ConfigOverrides::default())?;
    let conversation_manager = Arc::new(ConversationManager::new(AuthManager::shared(
        config.codex_home.clone(),
        config.preferred_auth_method,
    )));
    let config = Arc::new(config);

    let app = Router::new().route(
        "/ws",
        get({
            let manager = conversation_manager.clone();
            let cfg = config.clone();
            move |ws: WebSocketUpgrade| {
                let manager = manager.clone();
                let cfg = cfg.clone();
                async move { ws.on_upgrade(move |socket| handle_socket(socket, manager, cfg)) }
            }
        }),
    );

    let addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_socket(
    socket: WebSocket,
    conversation_manager: Arc<ConversationManager>,
    config: Arc<Config>,
) {
    let NewConversation {
        conversation_id: _,
        conversation,
        session_configured,
    } = match conversation_manager
        .new_conversation((*config).clone())
        .await
    {
        Ok(nc) => nc,
        Err(e) => {
            error!("failed to start conversation: {e:#}");
            return;
        }
    };

    let mut socket = socket;
    let synthetic_event = Event {
        id: "".to_string(),
        msg: EventMsg::SessionConfigured(session_configured),
    };
    if let Ok(s) = serde_json::to_string(&synthetic_event)
        && socket.send(Message::Text(s)).await.is_err() {
            return;
        }

    let (mut sender, mut receiver) = socket.split();
    let convo_for_incoming = conversation.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<Submission>(&text) {
                    Ok(sub) => {
                        if let Err(e) = convo_for_incoming.submit_with_id(sub).await {
                            error!("{e:#}");
                            break;
                        }
                    }
                    Err(e) => error!("invalid submission: {e}"),
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let convo_for_events = conversation;
    let send_task = tokio::spawn(async move {
        loop {
            match convo_for_events.next_event().await {
                Ok(event) => match serde_json::to_string(&event) {
                    Ok(s) => {
                        if sender.send(Message::Text(s)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => error!("Failed to serialize event: {e}"),
                },
                Err(e) => {
                    error!("{e:#}");
                    break;
                }
            }
        }
    });

    let _ = tokio::join!(recv_task, send_task);
}
