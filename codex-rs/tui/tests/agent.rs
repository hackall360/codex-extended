use tokio::sync::mpsc::unbounded_channel;

#[path = "../src/app_event.rs"]
mod app_event;
use app_event::AppEvent;

mod history_cell {
    use std::fmt::Debug;

    pub trait HistoryCell: Debug + Send + Sync {}
}

mod session_log {
    use super::AppEvent;
    pub fn log_inbound_app_event(_event: &AppEvent) {}
}

#[path = "../src/app_event_sender.rs"]
mod app_event_sender;
use app_event_sender::AppEventSender;

#[tokio::test(flavor = "current_thread")]
async fn forwards_error_events() {
    let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
    let tx = AppEventSender::new(tx_raw);
    tx.send(AppEvent::Error("boom".into()));
    match rx.recv().await {
        Some(AppEvent::Error(msg)) => assert_eq!(msg, "boom"),
        other => panic!("expected error event, got {:?}", other),
    }
}
