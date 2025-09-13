use crate::protocol::EventMsg;
use crate::protocol::RolloutItem;
use codex_protocol::models::ResponseItem;

/// Whether a rollout `item` should be persisted in rollout files.
#[inline]
pub(crate) fn is_persisted_response_item(item: &RolloutItem) -> bool {
    match item {
        RolloutItem::ResponseItem(item) => should_persist_response_item(item),
        RolloutItem::EventMsg(ev) => should_persist_event_msg(ev),
        // Persist Codex executive markers so we can analyze flows (e.g., compaction, API turns).
        RolloutItem::Compacted(_) | RolloutItem::TurnContext(_) | RolloutItem::SessionMeta(_) => {
            true
        }
    }
}

/// Whether a `ResponseItem` should be persisted in rollout files.
///
/// For extreme local logging and downstream analysis we now persist **all**
/// response items, including web search calls and any future variants. This
/// provides a full record of the model's structured outputs for fine‑tuning
/// or replay purposes.
#[inline]
pub(crate) fn should_persist_response_item(_item: &ResponseItem) -> bool {
    true
}

/// Whether an `EventMsg` should be persisted in rollout files.
///
/// To enable training on complete interaction histories we persist all runtime
/// events except [`ConversationPath`], which simply echoes the location of the
/// rollout file and adds no value to transcripts.
#[inline]
pub(crate) fn should_persist_event_msg(ev: &EventMsg) -> bool {
    !matches!(
        ev,
        EventMsg::ConversationPath(_) | EventMsg::SessionConfigured(_)
    )
}
