use serde_json::Value;
use serde_json::json;

pub(crate) fn default_lmstudio_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Codex CLI Final Response",
        "description": "Structured JSON response emitted by Codex CLI sessions.",
        "type": "object",
        "properties": {
            "status": {
                "description": "Overall completion state.",
                "type": "string",
                "enum": ["success", "partial", "blocked", "error"]
            },
            "summary": {
                "description": "Key bullet points summarizing the work performed.",
                "type": "array",
                "items": {
                    "type": "string",
                    "minLength": 1
                },
                "minItems": 1
            },
            "testing": {
                "description": "Tests or checks that were executed.",
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "description": "Exact command that was run.",
                            "type": "string",
                            "minLength": 1
                        },
                        "status": {
                            "description": "Outcome of the command.",
                            "type": "string",
                            "enum": ["pass", "fail", "not_run", "blocked"]
                        },
                        "details": {
                            "description": "Additional context about the run.",
                            "type": "string"
                        }
                    },
                    "required": ["command", "status"],
                    "additionalProperties": false
                }
            },
            "next_steps": {
                "description": "Follow-up work that should be considered.",
                "type": "array",
                "items": {
                    "type": "string",
                    "minLength": 1
                }
            },
            "notes": {
                "description": "Extra caveats or reminders for the user.",
                "type": "array",
                "items": {
                    "type": "string"
                }
            }
        },
        "required": ["summary"],
        "additionalProperties": false
    })
}
