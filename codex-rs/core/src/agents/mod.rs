use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;

pub mod builtin;

/// Context passed to agent handlers. Keep lightweight to avoid tight coupling
/// with internal session types.
#[derive(Clone)]
pub struct AgentContext<'a> {
    pub config: &'a Config,
}

/// Standard agent result payload. Structured as JSON string content that can
/// be returned via FunctionCallOutputPayload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub success: bool,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl AgentResult {
    pub fn ok(summary: impl Into<String>) -> Self {
        Self { success: true, summary: summary.into(), details: None }
    }
    pub fn with_details(summary: impl Into<String>, details: serde_json::Value) -> Self {
        Self { success: true, summary: summary.into(), details: Some(details) }
    }
    pub fn err(summary: impl Into<String>) -> Self {
        Self { success: false, summary: summary.into(), details: None }
    }
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Entrypoints used by tool-call handlers in codex.rs. Each returns an
/// AgentResult that is serialized back to the model.
pub async fn invoke_coding_agent(
    ctx: AgentContext<'_>,
    task: &str,
    model_role: Option<&str>,
) -> Result<AgentResult> {
    builtin::coding::run_coding_agent(ctx, task, model_role).await
}

pub async fn invoke_file_search_agent(
    ctx: AgentContext<'_>,
    query: &str,
    limit: Option<usize>,
    level: Option<&str>,
) -> Result<AgentResult> {
    builtin::file_search::run_file_search_agent(ctx, query, limit, level).await
}

pub async fn invoke_web_search_agent(
    ctx: AgentContext<'_>,
    query: &str,
    top_k: Option<usize>,
    mode: Option<&str>,
) -> Result<AgentResult> {
    builtin::web_search::run_web_search_agent(ctx, query, top_k, mode).await
}

pub async fn invoke_rag_agent(
    ctx: AgentContext<'_>,
    question: &str,
    top_k: Option<usize>,
    level: Option<&str>,
    include_web: bool,
    include_local: bool,
    answer_model_role: Option<&str>,
) -> Result<AgentResult> {
    builtin::rag::run_rag_agent(ctx, question, top_k, level, include_web, include_local, answer_model_role).await
}

pub async fn invoke_dag_agent(
    ctx: AgentContext<'_>,
    nodes: serde_json::Value,
    edges: serde_json::Value,
) -> Result<AgentResult> {
    #[derive(Deserialize)]
    struct Node { id: String, kind: String, #[serde(default)] params: serde_json::Value }
    #[derive(Deserialize)]
    struct Edge { from: String, to: String }
    let node_list: Vec<Node> = serde_json::from_value(nodes)?;
    let edge_list: Vec<Edge> = serde_json::from_value(edges)?;
    let mut indeg = std::collections::HashMap::<String, usize>::new();
    let mut adj = std::collections::HashMap::<String, Vec<String>>::new();
    for n in &node_list { indeg.entry(n.id.clone()).or_insert(0); }
    for e in &edge_list {
        adj.entry(e.from.clone()).or_default().push(e.to.clone());
        *indeg.entry(e.to.clone()).or_insert(0) += 1;
    }
    let mut q: std::collections::VecDeque<String> = indeg.iter().filter(|(_k,&v)| v==0).map(|(k,_v)| k.clone()).collect();
    let mut order = Vec::<String>::new();
    while let Some(u) = q.pop_front() {
        order.push(u.clone());
        if let Some(vs) = adj.get(&u) {
            for v in vs {
                if let Some(x) = indeg.get_mut(v) { *x -= 1; if *x==0 { q.push_back(v.clone()); } }
            }
        }
    }
    if order.len() != node_list.len() { return Ok(AgentResult::err("DAG has a cycle")); }

    let map: std::collections::HashMap<String, &Node> = node_list.iter().map(|n| (n.id.clone(), n)).collect();
    let mut results: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
    let mut logs: Vec<serde_json::Value> = Vec::new();
    for nid in order {
        let n = map.get(&nid).unwrap();
        let res = match n.kind.as_str() {
            "web_search" => {
                #[derive(Deserialize)] struct P { query: String, #[serde(default)] top_k: Option<usize>, #[serde(default)] mode: Option<String> }
                let p: P = serde_json::from_value(n.params.clone()).unwrap_or(P{ query: String::new(), top_k: None, mode: None });
                builtin::web_search::run_web_search_agent(ctx.clone(), &p.query, p.top_k, p.mode.as_deref()).await
            }
            "file_search" => {
                #[derive(Deserialize)] struct P { query: String, #[serde(default)] limit: Option<usize>, #[serde(default)] level: Option<String> }
                let p: P = serde_json::from_value(n.params.clone()).unwrap_or(P{ query: String::new(), limit: None, level: None });
                builtin::file_search::run_file_search_agent(ctx.clone(), &p.query, p.limit, p.level.as_deref()).await
            }
            "rag" => {
                #[derive(Deserialize)] struct P { question: String, #[serde(default)] top_k: Option<usize>, #[serde(default)] level: Option<String>, #[serde(default)] include_web: Option<bool>, #[serde(default)] include_local: Option<bool> }
                let p: P = serde_json::from_value(n.params.clone()).unwrap_or(P{ question: String::new(), top_k: None, level: None, include_web: Some(true), include_local: Some(true) });
                builtin::rag::run_rag_agent(ctx.clone(), &p.question, p.top_k, p.level.as_deref(), p.include_web.unwrap_or(true), p.include_local.unwrap_or(true)).await
            }
            "coding" => {
                #[derive(Deserialize)] struct P { task: String, #[serde(default)] model_role: Option<String> }
                let p: P = serde_json::from_value(n.params.clone()).unwrap_or(P{ task: String::new(), model_role: None });
                builtin::coding::run_coding_agent(ctx.clone(), &p.task, p.model_role.as_deref()).await
            }
            "autogen" => {
                #[derive(Deserialize)] struct P { prompt: String, #[serde(default)] provider_id: Option<String>, #[serde(default)] model: Option<String>, #[serde(default)] base_url: Option<String> }
                let p: P = serde_json::from_value(n.params.clone()).unwrap_or(P{ prompt: String::new(), provider_id: None, model: None, base_url: None });
                // Prefer provider_id from config map, otherwise base_url OSS provider.
                let model = p.model.unwrap_or_else(|| ctx.config.model.clone());
                if let Some(pid) = p.provider_id {
                    if let Some(provider) = ctx.config.model_providers.get(&pid) {
                        crate::autogen::run_autogen_with_provider(&p.prompt, provider, &model)
                            .await
                            .map(|out| AgentResult::ok(out))
                    } else {
                        Err(anyhow!(format!("unknown provider_id: {}", pid)))
                    }
                } else if let Some(base) = p.base_url {
                    let provider = crate::create_oss_provider_with_base_url(&base);
                    crate::autogen::run_autogen_with_provider(&p.prompt, &provider, &model)
                        .await
                        .map(|out| AgentResult::ok(out))
                } else {
                    // Fallback to default config provider
                    crate::autogen::run_autogen_with_provider(&p.prompt, &ctx.config.model_provider, &model)
                        .await
                        .map(|out| AgentResult::ok(out))
                }
            }
            other => Err(anyhow!(format!("unknown node kind: {other}"))),
        };
        match res {
            Ok(r) => { results.insert(nid.clone(), serde_json::json!({"summary": r.summary, "details": r.details})); logs.push(serde_json::json!({"node": nid, "ok": true})); },
            Err(e) => { logs.push(serde_json::json!({"node": nid, "ok": false, "error": e.to_string()})); },
        }
    }
    Ok(AgentResult::with_details("DAG executed", serde_json::json!({"results": results, "log": logs})))
}
