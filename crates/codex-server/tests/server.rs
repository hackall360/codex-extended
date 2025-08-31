use codex_core::Config;
use codex_server::{
    EmbeddingsRequest, EmbeddingsResponse, QueryRequest, QueryResponse, RagRequest, RagResponse,
    UpsertRequest, VectorRecord, start,
};
use httpmock::{Method::POST, MockServer};
use serde_json::json;

#[tokio::test]
async fn embed_upsert_query_rag_recovery() {
    let mock = MockServer::start();
    let _embed = mock.mock(|when, then| {
        when.method(POST).path("/api/embeddings");
        then.status(200)
            .json_body(json!({"embeddings": [[1.0,0.0]]}));
    });
    let _chat = mock.mock(|when, then| {
        when.method(POST).path("/api/chat");
        then.status(200)
            .json_body(json!({"message": {"role": "assistant", "content": "answer"}}));
    });

    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        backend_url: mock.base_url(),
        data_path: dir.path().to_path_buf(),
        port: 0,
        resp_port: None,
        ..Default::default()
    };
    let (addr, handle) = start(cfg.clone()).await.unwrap();

    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{}", addr.port());

    let emb: EmbeddingsResponse = client
        .post(format!("{base}/v1/embeddings"))
        .json(&EmbeddingsRequest {
            texts: vec!["doc1".into()],
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(emb.embeddings.len(), 1);

    let up = UpsertRequest {
        vectors: vec![VectorRecord {
            id: 1,
            values: vec![1.0, 0.0],
            document: "doc1".into(),
        }],
    };
    client
        .post(format!("{base}/v1/vector/upsert"))
        .json(&up)
        .send()
        .await
        .unwrap();

    let q: QueryResponse = client
        .post(format!("{base}/v1/vector/query"))
        .json(&QueryRequest {
            vector: vec![1.0, 0.0],
            top_k: 1,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(q.results[0].document, "doc1");

    let r: RagResponse = client
        .post(format!("{base}/v1/rag/answer"))
        .json(&RagRequest {
            question: "hello".into(),
            top_k: 1,
            tier: None,
            translate: Some(false),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r.contexts[0], "doc1");

    handle.abort();
    let _ = handle.await;

    let (addr2, handle2) = start(cfg).await.unwrap();
    let base2 = format!("http://127.0.0.1:{}", addr2.port());
    let q2: QueryResponse = client
        .post(format!("{base2}/v1/vector/query"))
        .json(&QueryRequest {
            vector: vec![1.0, 0.0],
            top_k: 1,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(q2.results[0].document, "doc1");

    handle2.abort();
    let _ = handle2.await;
}
