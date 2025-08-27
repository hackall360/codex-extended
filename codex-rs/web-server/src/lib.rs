use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
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
use codex_core::ConversationMetadata;
use codex_core::NewConversation;
use codex_core::config::Config;
use codex_core::config::ConfigOverrides;
use codex_core::config::load_config_as_toml;
use codex_core::protocol::InputItem as CoreInputItem;
use codex_core::protocol::Op;
use codex_login::AuthManager;
use codex_protocol::mcp_protocol::AddConversationListenerParams;
use codex_protocol::mcp_protocol::AddConversationSubscriptionResponse;
use codex_protocol::mcp_protocol::ConversationId;
use codex_protocol::mcp_protocol::InputItem as WireInputItem;
use codex_protocol::mcp_protocol::InterruptConversationParams;
use codex_protocol::mcp_protocol::InterruptConversationResponse;
use codex_protocol::mcp_protocol::NewConversationParams;
use codex_protocol::mcp_protocol::NewConversationResponse;
use codex_protocol::mcp_protocol::RemoveConversationListenerParams;
use codex_protocol::mcp_protocol::RemoveConversationSubscriptionResponse;
use codex_protocol::mcp_protocol::SendUserMessageParams;
use codex_protocol::mcp_protocol::SendUserMessageResponse;
use codex_protocol::mcp_protocol::SendUserTurnParams;
use codex_protocol::mcp_protocol::SendUserTurnResponse;
use codex_protocol::models::ResponseItem;
use futures::SinkExt;
use futures::StreamExt;
use futures::stream::SplitSink;
use mcp_types::JSONRPCError;
use mcp_types::JSONRPCErrorError;
use mcp_types::JSONRPCNotification;
use mcp_types::JSONRPCRequest;
use mcp_types::JSONRPCResponse;
use mcp_types::RequestId;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tracing::error;
use tracing::info;
use uuid::Uuid;

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
    let codex_home = config.codex_home.clone();
    let conversation_manager = ConversationManager::new(AuthManager::shared(
        codex_home.clone(),
        config.preferred_auth_method,
    ));

    let app =
        Router::new().route(
            "/ws",
            get({
                let manager = conversation_manager.clone();
                let codex_home = codex_home.clone();
                move |ws: WebSocketUpgrade| {
                    let manager = manager.clone();
                    let codex_home = codex_home.clone();
                    async move {
                        ws.on_upgrade(move |socket| handle_socket(socket, manager, codex_home))
                    }
                }
            }),
        );

    let addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Deserialize)]
struct CloseSessionParams {
    conversation_id: ConversationId,
}

#[derive(Serialize)]
struct ListSessionsResponse {
    sessions: Vec<ConversationId>,
}

#[derive(Deserialize)]
struct UpdateConfigParams {
    config: serde_json::Value,
}

#[derive(Deserialize)]
struct ExportSessionParams {
    conversation_id: ConversationId,
}

#[derive(Serialize)]
struct ExportSessionResponse {
    history: Vec<ResponseItem>,
    metadata: ConversationMetadata,
}

#[derive(Deserialize)]
struct ImportSessionParams {
    history: Vec<ResponseItem>,
    #[serde(default)]
    metadata: ConversationMetadata,
    #[serde(flatten)]
    new_params: NewConversationParams,
}

#[derive(Serialize)]
struct ImportSessionResponse {
    conversation_id: ConversationId,
    model: String,
}

async fn handle_socket(
    socket: WebSocket,
    conversation_manager: ConversationManager,
    codex_home: PathBuf,
) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    let mut listeners: HashMap<Uuid, oneshot::Sender<()>> = HashMap::new();

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            let Ok(req) = serde_json::from_str::<JSONRPCRequest>(&text) else {
                continue;
            };
            let id = req.id.clone();

            match req.method.as_str() {
                "newConversation" => {
                    let params: NewConversationParams = req
                        .params
                        .and_then(|v| serde_json::from_value(v).ok())
                        .unwrap_or_default();
                    handle_new_conversation(sender.clone(), &conversation_manager, id, params)
                        .await;
                }
                "addConversationListener" => {
                    let params: AddConversationListenerParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    handle_add_listener(
                        sender.clone(),
                        &conversation_manager,
                        &mut listeners,
                        id,
                        params,
                    )
                    .await;
                }
                "removeConversationListener" => {
                    let params: RemoveConversationListenerParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    handle_remove_listener(sender.clone(), &mut listeners, id, params).await;
                }
                "sendUserMessage" => {
                    let params: SendUserMessageParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    handle_send_user_message(sender.clone(), &conversation_manager, id, params)
                        .await;
                }
                "sendUserTurn" => {
                    let params: SendUserTurnParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    handle_send_user_turn(sender.clone(), &conversation_manager, id, params).await;
                }
                "interruptConversation" => {
                    let params: InterruptConversationParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    handle_interrupt(sender.clone(), &conversation_manager, id, params).await;
                }
                "listSessions" => {
                    let ids = conversation_manager.list_conversations().await;
                    let resp = ListSessionsResponse {
                        sessions: ids.into_iter().map(ConversationId).collect(),
                    };
                    send_response(sender.clone(), id, resp).await;
                }
                "closeSession" => {
                    let params: CloseSessionParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    let _ = conversation_manager
                        .remove_conversation(params.conversation_id.0)
                        .await;
                    send_response(sender.clone(), id, json!({})).await;
                }
                "exportSession" => {
                    let params: ExportSessionParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    match conversation_manager
                        .export_conversation(params.conversation_id.0)
                        .await
                    {
                        Ok((history, metadata)) => {
                            let resp = ExportSessionResponse { history, metadata };
                            send_response(sender.clone(), id, resp).await;
                        }
                        Err(err) => {
                            send_invalid_request(
                                sender.clone(),
                                id,
                                &format!("error exporting session: {err}"),
                            )
                            .await;
                        }
                    }
                }
                "importSession" => {
                    let params: ImportSessionParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    let config = match derive_config_from_params(params.new_params) {
                        Ok(c) => c,
                        Err(err) => {
                            send_invalid_request(
                                sender.clone(),
                                id,
                                &format!("error deriving config: {err}"),
                            )
                            .await;
                            continue;
                        }
                    };
                    match conversation_manager
                        .import_conversation(params.history, config, params.metadata)
                        .await
                    {
                        Ok(NewConversation {
                            conversation_id,
                            session_configured,
                            ..
                        }) => {
                            let resp = ImportSessionResponse {
                                conversation_id: ConversationId(conversation_id),
                                model: session_configured.model,
                            };
                            send_response(sender.clone(), id, resp).await;
                        }
                        Err(err) => {
                            send_invalid_request(
                                sender.clone(),
                                id,
                                &format!("error importing session: {err}"),
                            )
                            .await;
                        }
                    }
                }
                "getConfig" => {
                    let cfg = match load_config_as_toml(&codex_home) {
                        Ok(c) => c,
                        Err(err) => {
                            send_invalid_request(
                                sender.clone(),
                                id,
                                &format!("error loading config: {err}"),
                            )
                            .await;
                            continue;
                        }
                    };
                    let json_cfg = serde_json::to_value(cfg).unwrap_or_default();
                    send_response(sender.clone(), id, json_cfg).await;
                }
                "updateConfig" => {
                    let params: UpdateConfigParams =
                        match req.params.and_then(|v| serde_json::from_value(v).ok()) {
                            Some(p) => p,
                            None => {
                                send_invalid_request(sender.clone(), id, "missing params").await;
                                continue;
                            }
                        };
                    let mut current = match load_config_as_toml(&codex_home) {
                        Ok(c) => c,
                        Err(err) => {
                            send_invalid_request(
                                sender.clone(),
                                id,
                                &format!("error loading config: {err}"),
                            )
                            .await;
                            continue;
                        }
                    };
                    let updates = json_to_toml(params.config);
                    merge_toml(&mut current, updates);
                    if let Err(err) = write_config(&codex_home, &current) {
                        send_invalid_request(
                            sender.clone(),
                            id,
                            &format!("error saving config: {err}"),
                        )
                        .await;
                        continue;
                    }
                    send_response(sender.clone(), id, json!({})).await;
                }
                _ => {
                    send_invalid_request(sender.clone(), id, "method not found").await;
                }
            }
        }
    }

    for (_, tx) in listeners {
        let _ = tx.send(());
    }
}

async fn handle_new_conversation(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    conversation_manager: &ConversationManager,
    request_id: RequestId,
    params: NewConversationParams,
) {
    let metadata = ConversationMetadata {
        workspace_path: params.cwd.clone().map(PathBuf::from),
        custom_settings: params
            .config
            .clone()
            .map(|m| serde_json::Value::Object(m.into_iter().collect())),
    };
    let config = match derive_config_from_params(params) {
        Ok(c) => c,
        Err(err) => {
            send_invalid_request(sender, request_id, &format!("error deriving config: {err}"))
                .await;
            return;
        }
    };

    match conversation_manager
        .new_conversation_with_metadata(config, metadata)
        .await
    {
        Ok(NewConversation {
            conversation_id,
            session_configured,
            ..
        }) => {
            let response = NewConversationResponse {
                conversation_id: ConversationId(conversation_id),
                model: session_configured.model,
            };
            send_response(sender, request_id, response).await;
        }
        Err(err) => {
            send_invalid_request(
                sender,
                request_id,
                &format!("error creating conversation: {err}"),
            )
            .await;
        }
    }
}

async fn handle_add_listener(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    conversation_manager: &ConversationManager,
    listeners: &mut HashMap<Uuid, oneshot::Sender<()>>,
    request_id: RequestId,
    params: AddConversationListenerParams,
) {
    let AddConversationListenerParams { conversation_id } = params;
    let Ok(conversation) = conversation_manager
        .get_conversation(conversation_id.0)
        .await
    else {
        send_invalid_request(
            sender,
            request_id,
            &format!("conversation not found: {}", conversation_id.0),
        )
        .await;
        return;
    };

    let subscription_id = Uuid::new_v4();
    let (tx, mut rx) = oneshot::channel();
    listeners.insert(subscription_id, tx);
    let outgoing = sender.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut rx => { break; }
                evt = conversation.next_event() => {
                    let event = match evt { Ok(e) => e, Err(err) => { error!("{err}"); break; } };
                    let method = format!("codex/event/{}", event.msg);
                    let mut params = match serde_json::to_value(event.clone()) {
                        Ok(serde_json::Value::Object(map)) => map,
                        _ => continue,
                    };
                    params.insert("conversationId".to_string(), conversation_id.to_string().into());
                    let notification = JSONRPCNotification { jsonrpc: "2.0".to_string(), method, params: Some(params.into()) };
                    if let Ok(text) = serde_json::to_string(&notification) {
                        let mut guard = outgoing.lock().await;
                        if guard.send(Message::Text(text)).await.is_err() { break; }
                    }
                }
            }
        }
    });

    let response = AddConversationSubscriptionResponse { subscription_id };
    send_response(sender, request_id, response).await;
}

async fn handle_remove_listener(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    listeners: &mut HashMap<Uuid, oneshot::Sender<()>>,
    request_id: RequestId,
    params: RemoveConversationListenerParams,
) {
    let RemoveConversationListenerParams { subscription_id } = params;
    match listeners.remove(&subscription_id) {
        Some(cancel) => {
            let _ = cancel.send(());
            let response = RemoveConversationSubscriptionResponse {};
            send_response(sender, request_id, response).await;
        }
        None => {
            send_invalid_request(
                sender,
                request_id,
                &format!("subscription not found: {subscription_id}"),
            )
            .await;
        }
    }
}

async fn handle_send_user_message(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    conversation_manager: &ConversationManager,
    request_id: RequestId,
    params: SendUserMessageParams,
) {
    let SendUserMessageParams {
        conversation_id,
        items,
    } = params;
    let Ok(conversation) = conversation_manager
        .get_conversation(conversation_id.0)
        .await
    else {
        send_invalid_request(
            sender,
            request_id,
            &format!("conversation not found: {conversation_id}"),
        )
        .await;
        return;
    };

    let mapped: Vec<CoreInputItem> = items
        .into_iter()
        .map(|i| match i {
            WireInputItem::Text { text } => CoreInputItem::Text { text },
            WireInputItem::Image { image_url } => CoreInputItem::Image { image_url },
            WireInputItem::LocalImage { path } => CoreInputItem::LocalImage { path },
        })
        .collect();

    let _ = conversation.submit(Op::UserInput { items: mapped }).await;

    let response = SendUserMessageResponse {};
    send_response(sender, request_id, response).await;
}

async fn handle_send_user_turn(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    conversation_manager: &ConversationManager,
    request_id: RequestId,
    params: SendUserTurnParams,
) {
    let SendUserTurnParams {
        conversation_id,
        items,
        cwd,
        approval_policy,
        sandbox_policy,
        model,
        effort,
        summary,
    } = params;
    let Ok(conversation) = conversation_manager
        .get_conversation(conversation_id.0)
        .await
    else {
        send_invalid_request(
            sender,
            request_id,
            &format!("conversation not found: {conversation_id}"),
        )
        .await;
        return;
    };

    let mapped: Vec<CoreInputItem> = items
        .into_iter()
        .map(|i| match i {
            WireInputItem::Text { text } => CoreInputItem::Text { text },
            WireInputItem::Image { image_url } => CoreInputItem::Image { image_url },
            WireInputItem::LocalImage { path } => CoreInputItem::LocalImage { path },
        })
        .collect();

    let _ = conversation
        .submit(Op::UserTurn {
            items: mapped,
            cwd,
            approval_policy,
            sandbox_policy,
            model,
            effort,
            summary,
        })
        .await;

    let response = SendUserTurnResponse {};
    send_response(sender, request_id, response).await;
}

async fn handle_interrupt(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    conversation_manager: &ConversationManager,
    request_id: RequestId,
    params: InterruptConversationParams,
) {
    let InterruptConversationParams { conversation_id } = params;
    let Ok(conversation) = conversation_manager
        .get_conversation(conversation_id.0)
        .await
    else {
        send_invalid_request(
            sender,
            request_id,
            &format!("conversation not found: {conversation_id}"),
        )
        .await;
        return;
    };

    let _ = conversation.submit(Op::Interrupt).await;
    let response = InterruptConversationResponse {
        abort_reason: codex_core::protocol::TurnAbortReason::Interrupted,
    };
    send_response(sender, request_id, response).await;
}

async fn send_response<T: Serialize>(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    id: RequestId,
    result: T,
) {
    let response = JSONRPCResponse {
        id,
        jsonrpc: "2.0".to_string(),
        result: serde_json::to_value(result).unwrap_or_default(),
    };
    if let Ok(text) = serde_json::to_string(&response) {
        let mut guard = sender.lock().await;
        let _ = guard.send(Message::Text(text)).await;
    }
}

async fn send_invalid_request(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    id: RequestId,
    message: &str,
) {
    let error = JSONRPCErrorError {
        code: -32600,
        message: message.to_string(),
        data: None,
    };
    let response = JSONRPCError {
        id,
        jsonrpc: "2.0".to_string(),
        error,
    };
    if let Ok(text) = serde_json::to_string(&response) {
        let mut guard = sender.lock().await;
        let _ = guard.send(Message::Text(text)).await;
    }
}

fn merge_toml(dst: &mut toml::Value, src: toml::Value) {
    match (dst, src) {
        (toml::Value::Table(dst_tbl), toml::Value::Table(src_tbl)) => {
            for (k, v) in src_tbl {
                match dst_tbl.get_mut(&k) {
                    Some(existing) => merge_toml(existing, v),
                    None => {
                        dst_tbl.insert(k, v);
                    }
                }
            }
        }
        (dst_val, src_val) => {
            *dst_val = src_val;
        }
    }
}

fn write_config(codex_home: &Path, cfg: &toml::Value) -> std::io::Result<()> {
    std::fs::create_dir_all(codex_home)?;
    let config_path = codex_home.join("config.toml");
    let text = toml::to_string(cfg).map_err(|e| std::io::Error::other(e))?;
    std::fs::write(config_path, text)
}

fn json_to_toml(v: serde_json::Value) -> toml::Value {
    match v {
        serde_json::Value::Null => toml::Value::String(String::new()),
        serde_json::Value::Bool(b) => toml::Value::Boolean(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => toml::Value::String(s),
        serde_json::Value::Array(arr) => {
            toml::Value::Array(arr.into_iter().map(json_to_toml).collect())
        }
        serde_json::Value::Object(map) => {
            let tbl = map
                .into_iter()
                .map(|(k, v)| (k, json_to_toml(v)))
                .collect::<toml::value::Table>();
            toml::Value::Table(tbl)
        }
    }
}

fn derive_config_from_params(params: NewConversationParams) -> std::io::Result<Config> {
    let NewConversationParams {
        model,
        profile,
        cwd,
        approval_policy,
        sandbox: sandbox_mode,
        config: cli_overrides,
        workspace_paths,
        base_instructions,
        include_plan_tool,
        include_apply_patch_tool,
    } = params;
    let overrides = ConfigOverrides {
        model,
        config_profile: profile,
        cwd: cwd.map(PathBuf::from),
        approval_policy,
        sandbox_mode,
        model_provider: None,
        codex_linux_sandbox_exe: None,
        base_instructions,
        include_plan_tool,
        include_apply_patch_tool,
        disable_response_storage: None,
        show_raw_agent_reasoning: None,
        tools_web_search_request: None,
    };

    let mut cli_overrides = cli_overrides.unwrap_or_default();
    if let Some(paths) = workspace_paths {
        let arr = paths
            .into_iter()
            .map(|p| serde_json::Value::String(p.to_string_lossy().into_owned()))
            .collect();
        cli_overrides.insert(
            "sandbox_workspace_write.writable_roots".into(),
            serde_json::Value::Array(arr),
        );
    }

    let cli_overrides = cli_overrides
        .into_iter()
        .map(|(k, v)| (k, json_to_toml(v)))
        .collect();

    Config::load_with_cli_overrides(cli_overrides, overrides)
}
