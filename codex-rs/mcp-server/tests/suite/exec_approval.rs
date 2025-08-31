use codex_core::protocol::ReviewDecision;
use codex_mcp_server::ExecApprovalResponse;
use serde_json::json;

#[test]
fn exec_approval_response_deserializes() {
    let value = json!({
        "action": "submit",
        "content": { "decision": "approved" }
    });

    let response: ExecApprovalResponse = serde_json::from_value(value).unwrap();
    assert_eq!(response.action, "submit");
    let decision: ReviewDecision =
        serde_json::from_value(response.content["decision"].clone()).unwrap();
    assert_eq!(decision, ReviewDecision::Approved);
}
