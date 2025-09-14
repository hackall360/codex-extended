use codex_core::ContentItem;
use codex_core::ResponseEvent;
use codex_core::ResponseItem;
use codex_core::ToolBridge;
use codex_ollama::OllamaToolBridge;
use pretty_assertions::assert_eq;

#[test]
fn falls_back_to_message_when_json_invalid() {
    let bridge = OllamaToolBridge;
    let item = ResponseItem::Message {
        id: None,
        role: "assistant".into(),
        content: vec![ContentItem::OutputText {
            text: "not json".into(),
        }],
    };
    let events = bridge
        .decode_event(ResponseEvent::OutputItemDone(item))
        .expect("decode");
    match &events[0] {
        ResponseEvent::OutputItemDone(ResponseItem::Message { content, .. }) => {
            if let ContentItem::OutputText { text } = &content[0] {
                assert_eq!(text, "not json");
            } else {
                panic!("expected OutputText");
            }
        }
        _ => panic!("expected message"),
    }
}

#[test]
fn invalid_schema_returns_error() {
    let bridge = OllamaToolBridge;
    // Missing required name/input for type tool
    let item = ResponseItem::Message {
        id: None,
        role: "assistant".into(),
        content: vec![ContentItem::OutputText {
            text: "{\"type\":\"tool\"}".into(),
        }],
    };
    assert!(
        bridge
            .decode_event(ResponseEvent::OutputItemDone(item))
            .is_err()
    );
}

#[test]
fn recovers_after_plain_text() {
    let bridge = OllamaToolBridge;
    let plain = ResponseItem::Message {
        id: None,
        role: "assistant".into(),
        content: vec![ContentItem::OutputText { text: "hi".into() }],
    };
    let events = bridge
        .decode_event(ResponseEvent::OutputItemDone(plain))
        .expect("decode plain");
    assert_eq!(events.len(), 1);

    let tool = ResponseItem::Message {
        id: None,
        role: "assistant".into(),
        content: vec![ContentItem::OutputText {
            text: "{\"type\":\"tool\",\"name\":\"t\",\"input\":{}}".into(),
        }],
    };
    let events = bridge
        .decode_event(ResponseEvent::OutputItemDone(tool))
        .expect("decode tool");
    match &events[0] {
        ResponseEvent::OutputItemDone(ResponseItem::CustomToolCall { name, .. }) => {
            assert_eq!(name, "t");
        }
        _ => panic!("expected tool call"),
    }
}
