use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WebDoc {
    pub title: String,
    pub url: String,
    pub text: String,
}

/// Perform a DuckDuckGo HTML search via the lite endpoint and return a set of
/// result URLs.
pub async fn ddg_search(query: &str, limit: usize) -> Result<Vec<String>> {
    let url = "https://html.duckduckgo.com/html/";
    let params = [
        ("q", query),
        ("kl", "us-en"),
    ];
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (compatible; CodexBot/1.0)"),
    );
    let client = reqwest::Client::builder().default_headers(headers).build()?;
    let resp = client.post(url).form(&params).send().await?;
    let body = resp.text().await?;
    let doc = Html::parse_document(&body);
    let sel = Selector::parse("a.result__a, a.result__url").unwrap();
    let mut urls = Vec::new();
    for el in doc.select(&sel) {
        if let Some(href) = el.value().attr("href") {
            if href.starts_with("http") {
                urls.push(href.to_string());
            }
            if urls.len() >= limit {
                break;
            }
        }
    }
    urls.sort();
    urls.dedup();
    Ok(urls.into_iter().take(limit).collect())
}

/// Google Custom Search JSON API. Requires key + cx.
pub async fn google_cse_search(
    api_key: &str,
    cx: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<String>> {
    #[derive(Deserialize)]
    struct Item { link: String }
    #[derive(Deserialize)]
    struct Resp { items: Option<Vec<Item>> }
    let url = format!(
        "https://www.googleapis.com/customsearch/v1?key={api_key}&cx={cx}&q={}",
        urlencoding::encode(query)
    );
    let client = reqwest::Client::new();
    let body = client.get(url).send().await?.text().await?;
    let resp: Resp = serde_json::from_str(&body).map_err(|e| anyhow!("{e}: {body}"))?;
    let mut out = Vec::new();
    if let Some(items) = resp.items { for it in items { out.push(it.link) } }
    Ok(out.into_iter().take(limit).collect())
}

/// Fetch a URL and extract readable text content.
pub async fn fetch_text(url: &str) -> Result<WebDoc> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let resp = client.get(url).send().await?;
    let final_url = resp.url().to_string();
    let body = resp.text().await?;
    let html = Html::parse_document(&body);
    let title_sel = Selector::parse("title").unwrap();
    let title = html
        .select(&title_sel)
        .next()
        .map(|t| t.text().collect::<String>())
        .unwrap_or_else(|| final_url.clone());
    // Extract readable text by collecting text nodes from body
    let body_sel = Selector::parse("body").unwrap();
    let text = html
        .select(&body_sel)
        .flat_map(|b| b.text())
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(WebDoc { title, url: final_url, text })
}

/// Crawl pages starting from seed URLs up to `max_pages`.
pub async fn crawl_and_extract(urls: &[String], max_pages: usize) -> Result<Vec<WebDoc>> {
    let mut docs = Vec::new();
    for u in urls.iter().take(max_pages) {
        if let Ok(doc) = fetch_text(u).await { docs.push(doc); }
    }
    Ok(docs)
}

