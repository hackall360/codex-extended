use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
};

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use codex_core::Config;
use codex_ollama::{LlmTier, Message, OllamaClient, Role};
use codex_redis::Redis;
use codex_redis::resp::Resp;
use metrics::counter;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tracing::info;

static METRICS: OnceLock<PrometheusHandle> = OnceLock::new();

#[derive(Clone)]
pub struct AppState {
    pub ollama: Arc<OllamaClient>,
    pub redis: Arc<Redis>,
    pub metrics: PrometheusHandle,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/vector/upsert", post(vector_upsert))
        .route("/v1/vector/query", post(vector_query))
        .route("/v1/chat", post(chat))
        .route("/v1/rag/answer", post(rag_answer))
        .route("/v1/admin/compact", post(admin_compact))
        .route("/healthz", get(health))
        .route("/metrics", get(metrics_endpoint))
        .with_state(state)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingsRequest {
    pub texts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    pub embeddings: Vec<Vec<f32>>,
}

async fn embeddings(
    State(state): State<AppState>,
    Json(req): Json<EmbeddingsRequest>,
) -> Json<EmbeddingsResponse> {
    counter!("requests_total", 1, "endpoint" => "embeddings");
    if req.texts.len() > 256 {
        return Json(EmbeddingsResponse { embeddings: vec![] });
    }
    let embeddings = state.ollama.embed(&req.texts).await.unwrap_or_default();
    Json(EmbeddingsResponse { embeddings })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: u32,
    pub values: Vec<f32>,
    pub document: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertRequest {
    pub vectors: Vec<VectorRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertResponse {
    pub inserted: usize,
}

async fn vector_upsert(
    State(state): State<AppState>,
    Json(req): Json<UpsertRequest>,
) -> Json<UpsertResponse> {
    counter!("requests_total", 1, "endpoint" => "vector_upsert");
    if req.vectors.len() > 2000 {
        return Json(UpsertResponse { inserted: 0 });
    }
    if let Some(first) = req.vectors.first() {
        let dim = first.values.len();
        state.redis.execute(&vec![
            "VEC.CREATE".into(),
            "docs".into(),
            dim.to_string(),
            "4".into(),
            "10".into(),
        ]);
    }
    for v in &req.vectors {
        let vec = v
            .values
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",");
        state.redis.execute(&vec![
            "VEC.ADD".into(),
            "docs".into(),
            v.id.to_string(),
            vec,
            v.document.clone(),
        ]);
    }
    Json(UpsertResponse {
        inserted: req.vectors.len(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    pub vector: Vec<f32>,
    pub top_k: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryHit {
    pub id: u32,
    pub document: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub results: Vec<QueryHit>,
}

async fn vector_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    counter!("requests_total", 1, "endpoint" => "vector_query");
    let vec = req
        .vector
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let resp = state.redis.execute(&vec![
        "VEC.SEARCH".into(),
        "docs".into(),
        vec,
        req.top_k.to_string(),
        "10".into(),
    ]);
    let mut results = Vec::new();
    if let Resp::Array(Some(items)) = resp {
        for item in items {
            if let Resp::Array(Some(inner)) = item {
                if inner.len() >= 2 {
                    let id = match inner[0] {
                        Resp::Integer(i) => i as u32,
                        _ => 0,
                    };
                    let doc = match &inner[1] {
                        Resp::Bulk(Some(b)) => String::from_utf8_lossy(b).to_string(),
                        _ => String::new(),
                    };
                    results.push(QueryHit { id, document: doc });
                }
            }
        }
    }
    Json(QueryResponse { results })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub tier: Option<String>,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub reply: String,
}

fn parse_tier(t: Option<String>) -> LlmTier {
    match t.as_deref() {
        Some("low") => LlmTier::Low,
        Some("medium") => LlmTier::Medium,
        _ => LlmTier::High,
    }
}

async fn chat(State(state): State<AppState>, Json(req): Json<ChatRequest>) -> Json<ChatResponse> {
    counter!("requests_total", 1, "endpoint" => "chat");
    let tier = parse_tier(req.tier);
    let reply = state
        .ollama
        .chat(tier, &req.messages)
        .await
        .unwrap_or_default();
    Json(ChatResponse { reply })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RagRequest {
    pub question: String,
    pub top_k: usize,
    pub tier: Option<String>,
    pub translate: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub contexts: Vec<String>,
}

async fn rag_answer(
    State(state): State<AppState>,
    Json(req): Json<RagRequest>,
) -> Json<RagResponse> {
    counter!("requests_total", 1, "endpoint" => "rag_answer");
    let embed = state
        .ollama
        .embed(&[req.question.clone()])
        .await
        .unwrap_or_default();
    let query_vec = embed.get(0).cloned().unwrap_or_default();
    let vec = query_vec
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let resp = state.redis.execute(&vec![
        "VEC.SEARCH".into(),
        "docs".into(),
        vec,
        req.top_k.to_string(),
        "10".into(),
    ]);
    let mut contexts = Vec::new();
    if let Resp::Array(Some(items)) = resp {
        for item in items {
            if let Resp::Array(Some(inner)) = item {
                if inner.len() >= 2 {
                    if let Resp::Bulk(Some(b)) = &inner[1] {
                        contexts.push(String::from_utf8_lossy(b).to_string());
                    }
                }
            }
        }
    }
    let mut messages = Vec::new();
    if !contexts.is_empty() {
        messages.push(Message {
            role: Role::System,
            content: format!("Use the following context:\n{}", contexts.join("\n")),
        });
    }
    messages.push(Message {
        role: Role::User,
        content: req.question,
    });
    let tier = parse_tier(req.tier);
    let answer = state.ollama.chat(tier, &messages).await.unwrap_or_default();
    Json(RagResponse { answer, contexts })
}

async fn admin_compact(State(state): State<AppState>) -> &'static str {
    counter!("requests_total", 1, "endpoint" => "admin_compact");
    state.redis.execute(&vec!["COMPACT".into()]);
    "ok"
}

async fn health() -> &'static str {
    "ok"
}

async fn metrics_endpoint(State(state): State<AppState>) -> String {
    state.metrics.render()
}

pub async fn start(
    cfg: Config,
) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn std::error::Error>> {
    let recorder = METRICS
        .get_or_init(|| {
            PrometheusBuilder::new()
                .install_recorder()
                .expect("metrics")
        })
        .clone();
    let _ = tracing_subscriber::fmt::try_init();
    let ollama = Arc::new(OllamaClient::new(cfg.backend_url.clone())?);
    // startup checks
    ollama.embed(&["start".into()]).await?;
    ollama
        .chat(
            LlmTier::Low,
            &[Message {
                role: Role::User,
                content: "ping".into(),
            }],
        )
        .await?;

    let redis = Arc::new(Redis::new(Some(cfg.data_path.join("aof.log"))));
    if let Some(port) = cfg.resp_port {
        let r = redis.clone();
        tokio::spawn(async move {
            let addr = format!("0.0.0.0:{}", port);
            let _ = r.listen(&addr).await;
        });
    }

    let state = AppState {
        ollama: ollama.clone(),
        redis: redis.clone(),
        metrics: recorder.clone(),
    };
    let app = router(state);
    let listener = TcpListener::bind(("0.0.0.0", cfg.port)).await?;
    let addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    info!("listening on {}", addr);
    Ok((addr, handle))
}
