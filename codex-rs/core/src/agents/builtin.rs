use std::sync::Arc;

use anyhow::{Context, Result};
use codex_login::AuthManager;
use serde::{Deserialize, Serialize};

use crate::agents::{AgentContext, AgentResult};
use crate::codex::Codex;
use crate::codex::CodexSpawnOk;
use crate::config::Config;
use crate::config::find_codex_home;
use crate::model_family::{built_in_model_capabilities, find_family_for_model};
use crate::protocol::EventMsg;
use crate::protocol::InputItem;
use crate::protocol::Op;

pub mod coding {
    use super::*;

    #[derive(Debug, Clone, Deserialize)]
    pub struct CodingAgentArgs {
        pub task: String,
        #[serde(default)]
        pub model_role: Option<String>,
    }

    pub async fn run_coding_agent(
        ctx: AgentContext<'_>,
        task: &str,
        model_role: Option<&str>,
    ) -> Result<AgentResult> {
        let cfg =
            clone_config_with_optional_role(ctx.config, model_role).context("config clone")?;

        let codex_home = find_codex_home().context("find_codex_home")?;
        let auth_manager = AuthManager::shared(codex_home, cfg.preferred_auth_method);

        let CodexSpawnOk { codex, .. } = Codex::spawn(cfg, auth_manager, None)
            .await
            .context("spawn coding agent session")?;

        // Submit the task as a user message.
        codex
            .submit(Op::UserInput {
                items: vec![InputItem::Text {
                    text: task.to_string(),
                }],
            })
            .await
            .context("submit user input to coding agent")?;

        // Drain until TaskComplete, capturing the last assistant message.
        let mut last_msg: Option<String> = None;
        loop {
            let ev = codex.next_event().await.context("next_event")?;
            match ev.msg {
                EventMsg::AgentMessage(m) => {
                    last_msg = Some(m.message);
                }
                EventMsg::TaskComplete { .. } => break,
                _ => {}
            }
        }

        let summary = last_msg.unwrap_or_else(|| "coding agent completed".to_string());
        Ok(AgentResult::ok(summary))
    }

    fn clone_config_with_optional_role(base: &Config, role: Option<&str>) -> Result<Config> {
        let mut cfg = base.clone();
        if let Some(role_name) = role {
            if let Some(role) = base.model_roles.get(role_name) {
                cfg.model = role.model.clone();
                cfg.model_family = find_family_for_model(&cfg.model, built_in_model_capabilities())
                    .unwrap_or_else(|| base.model_family.clone());
                if let Some(provider_id) = &role.provider {
                    cfg.model_provider_id = provider_id.clone();
                    if let Some(info) = base.model_providers.get(provider_id) {
                        cfg.model_provider = info.clone();
                    }
                }
            }
        }
        Ok(cfg)
    }
}

pub mod file_search {
    use super::*;
    use codex_file_search::{FileSearchResults, run};
    use std::num::NonZero;
    use std::path::Path;
    use std::sync::Arc as StdArc;
    use std::sync::atomic::AtomicBool;

    #[derive(Debug, Clone, Deserialize)]
    pub struct FileSearchArgs {
        pub query: String,
        #[serde(default)]
        pub limit: Option<usize>,
    }

    pub async fn run_file_search_agent(
        ctx: AgentContext<'_>,
        query: &str,
        limit: Option<usize>,
    ) -> Result<AgentResult> {
        let search_dir = &ctx.config.cwd;
        let limit = NonZero::new(limit.unwrap_or(50)).unwrap_or(NonZero::new(50).unwrap());
        let cancel = StdArc::new(AtomicBool::new(false));
        let FileSearchResults {
            matches,
            total_match_count,
        } = run(
            query,
            limit,
            Path::new(search_dir),
            vec![],
            NonZero::new(8).unwrap(),
            cancel,
            true,
        )?;
        let details = serde_json::json!({
            "total": total_match_count,
            "matches": matches,
        });
        Ok(AgentResult::with_details("file search complete", details))
    }
}

pub mod web_search {
    use super::*;
    use crate::web::{crawl_and_extract, ddg_search, google_cse_search};

    #[derive(Debug, Clone, Deserialize)]
    pub struct WebSearchArgs {
        pub query: String,
        #[serde(default)]
        pub top_k: Option<usize>,
        #[serde(default)]
        pub mode: Option<String>, // normal | deep | research
    }

    pub async fn run_web_search_agent(
        _ctx: AgentContext<'_>,
        query: &str,
        top_k: Option<usize>,
        mode: Option<&str>,
    ) -> Result<AgentResult> {
        let provider =
            std::env::var("CODEX_WEBSEARCH_PROVIDER").unwrap_or_else(|_| "duckduckgo".to_string());
        let k = top_k.unwrap_or(5);
        let urls = match provider.as_str() {
            "google_cse" => {
                let key = std::env::var("CODEX_GOOGLE_CSE_KEY")
                    .map_err(|_| anyhow!("CODEX_GOOGLE_CSE_KEY not set"))?;
                let cx = std::env::var("CODEX_GOOGLE_CSE_CX")
                    .map_err(|_| anyhow!("CODEX_GOOGLE_CSE_CX not set"))?;
                google_cse_search(&key, &cx, query, k).await?
            }
            _ => ddg_search(query, k).await?,
        };

        let depth = match mode.unwrap_or("normal") {
            "deep" => k * 2,
            "research" => k * 4,
            _ => k,
        };
        let docs = crawl_and_extract(&urls, depth).await?;
        let details = serde_json::json!({
            "query": query,
            "provider": provider,
            "documents": docs,
        });
        Ok(AgentResult::with_details("web search complete", details))
    }
}

pub mod rag {
    use super::*;
    use crate::web::{WebDoc, crawl_and_extract, ddg_search};
    use std::fs;
    use std::path::Path;

    #[derive(Debug, Clone, Deserialize)]
    pub struct RagArgs {
        pub question: String,
        #[serde(default)]
        pub top_k: Option<usize>,
        #[serde(default)]
        pub level: Option<String>, // local search level
        #[serde(default)]
        pub include_web: Option<bool>,
        #[serde(default)]
        pub include_local: Option<bool>,
    }

    pub async fn run_rag_agent(
        ctx: AgentContext<'_>,
        question: &str,
        top_k: Option<usize>,
        level: Option<&str>,
        include_web: bool,
        include_local: bool,
        answer_model_role: Option<&str>,
    ) -> Result<AgentResult> {
        let k = top_k.unwrap_or(5);
        let mut contexts: Vec<(String, String)> = Vec::new();

        if include_local {
            let roots = match level.unwrap_or("codebase") {
                "small" => vec![ctx.config.cwd.clone()],
                "medium" | "codebase" => vec![ctx.config.cwd.clone()],
                _ => vec![ctx.config.cwd.clone()],
            };
            let mut added = 0usize;
            for root in roots {
                for entry in walkdir::WalkDir::new(root)
                    .max_depth(5)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let p = entry.path();
                    if p.is_file() && added < 200 {
                        if let Ok(content) = fs::read_to_string(p) {
                            if content.chars().any(|c| c == '\u{0}') {
                                continue;
                            }
                            contexts.push((p.display().to_string(), content));
                            added += 1;
                        }
                    }
                }
            }
        }

        if include_web {
            let urls = ddg_search(question, k).await.unwrap_or_default();
            let docs: Vec<WebDoc> = crawl_and_extract(&urls, k).await.unwrap_or_default();
            for d in docs {
                contexts.push((d.url, d.text));
            }
        }

        // Prefer config field (future), then env var for embeddings endpoint.
        let base = std::env::var("CODEX_SERVER_URL").ok();
        let client = reqwest::Client::new();
        let mut scored: Vec<(f32, String, String)> = Vec::new();
        if let Some(base) = base {
            let q_vec = embed_texts(&client, &base, &[question.to_string()])
                .await?
                .pop()
                .unwrap_or_default();
            for (prov, text) in contexts.iter() {
                for chunk in chunk_text(text, 2000) {
                    let v = embed_texts(&client, &base, &[chunk.clone()])
                        .await?
                        .pop()
                        .unwrap_or_default();
                    let sim = cosine_sim(&q_vec, &v);
                    scored.push((sim, prov.clone(), chunk));
                }
            }
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        } else {
            for (prov, text) in contexts.iter() {
                let score = text.matches(question).count() as f32;
                for chunk in chunk_text(text, 2000) {
                    scored.push((score, prov.clone(), chunk));
                }
            }
            scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        }
        let top = scored.into_iter().take(k).collect::<Vec<_>>();

        // Synthesize answer via nested Codex turn
        let mut sys = String::from("Use the following context to answer concisely.\n\n");
        for (i, (_s, prov, chunk)) in top.iter().enumerate() {
            sys.push_str(&format!("[{}] {}\n{}\n\n", i + 1, prov, chunk));
        }
        let codex_home = crate::config::find_codex_home()?;
        let auth_manager = AuthManager::shared(codex_home, ctx.config.preferred_auth_method);
        let mut nested_cfg = ctx.config.clone();
        // Allow answer synthesis model override using a model_role mapping.
        if let Some(role_name) = answer_model_role {
            if let Some(role) = ctx.config.model_roles.get(role_name) {
                nested_cfg.model = role.model.clone();
                nested_cfg.model_family =
                    find_family_for_model(&nested_cfg.model, built_in_model_capabilities())
                        .unwrap_or_else(|| ctx.config.model_family.clone());
                if let Some(provider_id) = &role.provider {
                    nested_cfg.model_provider_id = provider_id.clone();
                    if let Some(info) = ctx.config.model_providers.get(provider_id) {
                        nested_cfg.model_provider = info.clone();
                    }
                }
            }
        }
        nested_cfg.base_instructions = Some(sys.clone());
        let CodexSpawnOk { codex, .. } = Codex::spawn(nested_cfg, auth_manager, None).await?;
        codex
            .submit(codex_protocol::protocol::Op::UserInput {
                items: vec![codex_protocol::protocol::InputItem::Text {
                    text: question.to_string(),
                }],
            })
            .await?;
        let mut answer: Option<String> = None;
        loop {
            let ev = codex.next_event().await?;
            match ev.msg {
                codex_protocol::protocol::EventMsg::AgentMessage(m) => answer = Some(m.message),
                codex_protocol::protocol::EventMsg::TaskComplete { .. } => break,
                _ => {}
            }
        }
        let details = serde_json::json!({
            "contexts": top.iter().map(|(_, prov, chunk)| serde_json::json!({"provenance": prov, "text": chunk})).collect::<Vec<_>>()
        });
        Ok(AgentResult {
            success: true,
            summary: answer.unwrap_or_else(|| "RAG answer available".to_string()),
            details: Some(details),
        })
    }

    async fn embed_texts(
        client: &reqwest::Client,
        base_url: &str,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>> {
        #[derive(Serialize)]
        struct Req {
            texts: Vec<String>,
        }
        #[derive(Deserialize)]
        struct Resp {
            embeddings: Vec<Vec<f32>>,
        }
        let url = format!("{}/v1/embeddings", base_url.trim_end_matches('/'));
        let resp: Resp = client
            .post(url)
            .json(&Req {
                texts: texts.to_vec(),
            })
            .send()
            .await?
            .json()
            .await?;
        Ok(resp.embeddings)
    }

    fn chunk_text(s: &str, max_chars: usize) -> Vec<String> {
        if s.len() <= max_chars {
            return vec![s.to_string()];
        };
        let mut out = Vec::new();
        let mut start = 0usize;
        while start < s.len() {
            let end = (start + max_chars).min(s.len());
            out.push(s[start..end].to_string());
            start = end;
        }
        out
    }

    fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
        if a.is_empty() || b.is_empty() || a.len() != b.len() {
            return 0.0;
        };
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}
