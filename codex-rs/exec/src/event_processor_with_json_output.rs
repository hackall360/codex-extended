use std::collections::HashMap;
use std::path::PathBuf;

use codex_core::config::Config;
use codex_core::protocol::Event;
use codex_core::protocol::EventMsg;
use codex_core::protocol::TaskCompleteEvent;
use codex_protocol::models::ResponseItem;
use serde_json::json;

use crate::event_processor::CodexStatus;
use crate::event_processor::EventProcessor;
use crate::event_processor::handle_last_message;
use codex_common::create_config_summary_entries;

pub(crate) struct EventProcessorWithJsonOutput {
    last_message_path: Option<PathBuf>,
}

impl EventProcessorWithJsonOutput {
    pub fn new(last_message_path: Option<PathBuf>) -> Self {
        Self { last_message_path }
    }
}

impl EventProcessor for EventProcessorWithJsonOutput {
    fn print_config_summary(&mut self, config: &Config, prompt: &str) {
        let entries = create_config_summary_entries(config)
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<HashMap<String, String>>();
        #[expect(clippy::expect_used)]
        let config_json =
            serde_json::to_string(&entries).expect("Failed to serialize config summary to JSON");
        println!("{config_json}");

        let prompt_json = json!({
            "prompt": prompt,
        });
        println!("{prompt_json}");
    }

    fn process_event(&mut self, event: Event) -> CodexStatus {
        match event.msg {
            EventMsg::AgentMessageDelta(_) | EventMsg::AgentReasoningDelta(_) => {
                // Suppress streaming events in JSON mode.
                CodexStatus::Running
            }
            EventMsg::TaskComplete(TaskCompleteEvent { last_agent_message }) => {
                if let Some(output_file) = self.last_message_path.as_deref() {
                    handle_last_message(last_agent_message.as_deref(), output_file);
                }
                CodexStatus::InitiateShutdown
            }
            EventMsg::ShutdownComplete => CodexStatus::Shutdown,
            EventMsg::GetHistoryEntryResponse(ev) => {
                if let Some(entry) = ev.entry
                    && let Ok(item) = serde_json::from_str::<ResponseItem>(&entry.text)
                        && let Ok(line) = serde_json::to_string(&item) {
                            println!("{line}");
                        }
                CodexStatus::Running
            }
            _ => {
                if let Ok(line) = serde_json::to_string(&event) {
                    println!("{line}");
                }
                CodexStatus::Running
            }
        }
    }
}
