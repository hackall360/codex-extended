use omni_bridge::{Adapter, Bridge, adapters::{openai::OpenAIAdapter, anthropic::AnthropicAdapter}, OmniError};
use serde_json::json;

fn bridge() -> Bridge {
    Bridge::new()
        .with_adapter(OpenAIAdapter::new())
        .with_adapter(AnthropicAdapter::new())
}

#[test]
fn translate_anthropic_request_to_openai() {
    let b = bridge();
    let anthropic_req = json!({
        "model":"claude-3-5-sonnet",
        "system":"Be terse.",
        "messages":[{"role":"user","content":[{"type":"text","text":"ping"}]}],
        "max_tokens":128
    });
    let openai_req = b.translate_request("anthropic","openai",&anthropic_req).unwrap();
    assert_eq!(openai_req["model"], json!("claude-3-5-sonnet"));
    let msgs = openai_req["messages"].as_array().unwrap();
    assert_eq!(msgs[0]["role"], "system");
    assert_eq!(msgs[1]["content"].as_array().unwrap()[0]["text"], "ping");
}

#[test]
fn translate_openai_response_to_anthropic() {
    let b = bridge();
    let openai_resp = json!({
        "choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":[{"type":"text","text":"pong"}]}}],
        "usage":{"prompt_tokens":5,"completion_tokens":5,"total_tokens":10}
    });
    let anth_resp = b.translate_response("openai","anthropic",&openai_resp).unwrap();
    assert_eq!(anth_resp["content"].as_array().unwrap()[0]["text"], "pong");
    assert_eq!(anth_resp["stop_reason"], "end_turn");
}

#[test]
fn streaming_openai_to_anthropic() {
    let openai = OpenAIAdapter::new();
    let anth = AnthropicAdapter::new();
    let chunk = b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":null}]}\n\n";
    let finish = b"data: [DONE]\n\n";
    let mut deltas = openai.stream_decode(chunk).unwrap();
    deltas.extend(openai.stream_decode(finish).unwrap());
    let events = anth.stream_encode(&deltas).unwrap();
    let events: Vec<String> = events.into_iter().map(|e| String::from_utf8_lossy(&e).into_owned()).collect();
    assert!(events.iter().any(|e| e.contains("content_block_delta")));
    assert!(events.iter().any(|e| e.contains("message_stop")));
}

#[test]
fn translate_unknown_provider_fails() {
    let b = bridge();
    let err = b.translate_request("openai","unknown",&json!({})).unwrap_err();
    assert!(matches!(err, OmniError::Unsupported(_)));
}

#[test]
fn translate_openai_tool_response_to_anthropic() {
    let b = bridge();
    let openai_resp = json!({
        "choices":[{"index":0,"finish_reason":"tool_calls","message":{
            "role":"assistant",
            "content":[],
            "tool_calls":[{"id":"call_1","type":"function","function":{"name":"do_it","arguments":"{\"x\":1}"}}]
        }}],
        "usage":{}
    });
    let anth_resp = b.translate_response("openai","anthropic",&openai_resp).unwrap();
    let block = &anth_resp["content"].as_array().unwrap()[0];
    assert_eq!(block["name"], "do_it");
    assert_eq!(anth_resp["stop_reason"], "tool_use");
}
