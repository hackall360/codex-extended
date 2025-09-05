use crate::tool_apply_patch::ApplyPatchToolType;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Deserialize)]
pub struct ModelCapabilities {
    #[serde(default)]
    pub needs_special_apply_patch_instructions: bool,
    #[serde(default)]
    pub supports_reasoning_summaries: bool,
    #[serde(default)]
    pub uses_local_shell_tool: bool,
    #[serde(default)]
    pub apply_patch_tool_type: Option<ApplyPatchToolType>,
}

/// A model family is a group of models that share certain characteristics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelFamily {
    /// The full model slug used to derive this model family, e.g.
    /// "gpt-4.1-2025-04-14".
    pub slug: String,

    /// The model family name, e.g. "gpt-4.1". Note this should able to be used
    /// with [`crate::openai_model_info::get_model_info`].
    pub family: String,

    /// True if the model needs additional instructions on how to use the
    /// "virtual" `apply_patch` CLI.
    pub needs_special_apply_patch_instructions: bool,

    // Whether the `reasoning` field can be set when making a request to this
    // model family. Note it has `effort` and `summary` subfields (though
    // `summary` is optional).
    pub supports_reasoning_summaries: bool,

    // This should be set to true when the model expects a tool named
    // "local_shell" to be provided. Its contract must be understood natively by
    // the model such that its description can be omitted.
    // See https://platform.openai.com/docs/guides/tools-local-shell
    pub uses_local_shell_tool: bool,

    /// Present if the model performs better when `apply_patch` is provided as
    /// a tool call instead of just a bash command
    pub apply_patch_tool_type: Option<ApplyPatchToolType>,
}

static BUILT_IN_MODEL_CAPABILITIES: Lazy<HashMap<String, ModelCapabilities>> =
    Lazy::new(|| {
        use ApplyPatchToolType::*;
        let mut m = HashMap::new();
        m.insert(
            "o3".into(),
            ModelCapabilities {
                supports_reasoning_summaries: true,
                ..Default::default()
            },
        );
        m.insert(
            "o4-mini".into(),
            ModelCapabilities {
                supports_reasoning_summaries: true,
                ..Default::default()
            },
        );
        m.insert(
            "codex-mini-latest".into(),
            ModelCapabilities {
                supports_reasoning_summaries: true,
                uses_local_shell_tool: true,
                ..Default::default()
            },
        );
        // Fallback for any slug beginning with "codex-".
        m.insert(
            "codex-".into(),
            ModelCapabilities {
                supports_reasoning_summaries: true,
                ..Default::default()
            },
        );
        m.insert(
            "gpt-4.1".into(),
            ModelCapabilities {
                needs_special_apply_patch_instructions: true,
                ..Default::default()
            },
        );
        m.insert(
            "gpt-oss".into(),
            ModelCapabilities {
                apply_patch_tool_type: Some(Function),
                ..Default::default()
            },
        );
        m.insert(
            "gpt-5".into(),
            ModelCapabilities {
                supports_reasoning_summaries: true,
                ..Default::default()
            },
        );
        m
    });

pub fn built_in_model_capabilities() -> &'static HashMap<String, ModelCapabilities> {
    &BUILT_IN_MODEL_CAPABILITIES
}

/// Returns a `ModelFamily` for the given model slug, or `None` if the slug
/// does not match any known model family.
pub fn find_family_for_model(
    slug: &str,
    model_capabilities: &HashMap<String, ModelCapabilities>,
) -> Option<ModelFamily> {
    let family = if slug.starts_with("o3") {
        "o3"
    } else if slug.starts_with("o4-mini") {
        "o4-mini"
    } else if slug.starts_with("codex-mini-latest") {
        "codex-mini-latest"
    } else if slug.starts_with("codex-") {
        slug
    } else if slug.starts_with("gpt-4.1") {
        "gpt-4.1"
    } else if slug.starts_with("gpt-oss") {
        "gpt-oss"
    } else if slug.starts_with("gpt-4o") {
        "gpt-4o"
    } else if slug.starts_with("gpt-3.5") {
        "gpt-3.5"
    } else if slug.starts_with("gpt-5") {
        "gpt-5"
    } else {
        return None;
    };

    let caps = model_capabilities
        .get(slug)
        .or_else(|| model_capabilities.get(family))
        .or_else(|| {
            if slug.starts_with("codex-") {
                model_capabilities.get("codex-")
            } else {
                None
            }
        })
        .cloned()
        .unwrap_or_default();

    Some(ModelFamily {
        slug: slug.to_string(),
        family: family.to_string(),
        needs_special_apply_patch_instructions: caps.needs_special_apply_patch_instructions,
        supports_reasoning_summaries: caps.supports_reasoning_summaries,
        uses_local_shell_tool: caps.uses_local_shell_tool,
        apply_patch_tool_type: caps.apply_patch_tool_type,
    })
}

