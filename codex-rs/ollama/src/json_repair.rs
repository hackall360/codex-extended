use serde_json::Map;
use serde_json::Value;

// Extract a plausible JSON object from text: strip code fences and take the
// first balanced {...} object if present; otherwise return the trimmed input.
pub fn extract_json_candidate(text: &str) -> String {
    let s = text.trim();
    if let Some(stripped) = strip_code_fence(s) {
        if let Some(obj) = extract_balanced_object(stripped) {
            return obj;
        }
        return stripped.trim().to_string();
    }
    if let Some(obj) = extract_balanced_object(s) {
        return obj;
    }
    s.to_string()
}

// Attempt to coerce common near-miss shapes into our schema
// {"type":"tool","name":"...","input":{...}}
// or {"type":"message","content":"..."}
pub fn coerce_to_schema(mut value: Value) -> Option<Value> {
    let obj = value.as_object_mut()?;

    let has_type = obj.get("type").and_then(Value::as_str).is_some();
    if !has_type {
        // Try to detect a tool call
        // Variants seen from models: { name, arguments }, { tool, input },
        // { function_call: { name, arguments } }
        if obj.get("name").is_some()
            || obj.get("tool").is_some()
            || obj.get("function_call").is_some()
            || obj.get("command").is_some()
        {
            if let Some(coerced) = try_coerce_from_command_array(obj) {
                return Some(coerced);
            }
            let (name, input) = infer_tool_parts(obj);
            if let Some(name) = name {
                obj.insert("type".to_string(), Value::String("tool".to_string()));
                obj.insert("name".to_string(), Value::String(name));
                obj.insert("input".to_string(), input);
                return Some(Value::Object(obj.clone()));
            }
        }

        // Maybe it's a plain message
        if let Some(content) = obj
            .get("content")
            .and_then(|v| if v.is_string() { Some(v.clone()) } else { None })
            .or_else(|| obj.get("text").cloned())
        {
            let mut m = Map::new();
            m.insert("type".to_string(), Value::String("message".to_string()));
            m.insert(
                "content".to_string(),
                match content {
                    Value::String(s) => Value::String(s),
                    other => Value::String(other.to_string()),
                },
            );
            return Some(Value::Object(m));
        }
    } else if let Some(t) = obj.get("type").and_then(Value::as_str)
        && t == "tool"
    {
        // Normalize aliases for input
        if obj.get("input").is_none() {
            if let Some(input) = obj
                .remove("arguments")
                .or_else(|| obj.remove("args"))
                .or_else(|| obj.remove("parameters"))
                .or_else(|| obj.remove("params"))
            {
                let input = normalize_possible_json_string(input);
                obj.insert("input".to_string(), input);
            }
        } else if let Some(v) = obj.remove("input") {
            obj.insert("input".to_string(), normalize_possible_json_string(v));
        }
        return Some(Value::Object(obj.clone()));
    }

    None
}

fn infer_tool_parts(obj: &Map<String, Value>) -> (Option<String>, Value) {
    // name candidates
    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .or_else(|| {
            obj.get("tool")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        })
        .or_else(|| {
            obj.get("function_call")
                .and_then(Value::as_object)
                .and_then(|m| m.get("name").and_then(Value::as_str))
                .map(|s| s.to_string())
        });

    // input candidates
    let input = obj
        .get("input")
        .cloned()
        .or_else(|| obj.get("arguments").cloned())
        .or_else(|| obj.get("args").cloned())
        .or_else(|| obj.get("parameters").cloned())
        .or_else(|| obj.get("params").cloned())
        .or_else(|| {
            obj.get("function_call")
                .and_then(Value::as_object)
                .and_then(|m| m.get("arguments").cloned())
        })
        .map(normalize_possible_json_string)
        .unwrap_or(Value::Null);

    (name, input)
}

fn try_coerce_from_command_array(obj: &Map<String, Value>) -> Option<Value> {
    let cmd = obj.get("command")?.as_array()?;
    if cmd.is_empty() {
        return None;
    }
    let first = cmd.first()?;
    let tool = first.as_str()?;
    match tool {
        "apply_patch" => {
            let patch_val = cmd.get(1).cloned().unwrap_or(Value::Null);
            let patch_str = match patch_val {
                Value::String(s) => s,
                Value::Array(a) => a
                    .into_iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            };
            let patch = if patch_str.contains("*** Begin Patch") {
                patch_str
            } else {
                extract_patch_envelope(&patch_str).unwrap_or(patch_str)
            };
            Some(Value::Object(Map::from_iter([
                ("type".to_string(), Value::String("tool".to_string())),
                ("name".to_string(), Value::String("apply_patch".to_string())),
                (
                    "input".to_string(),
                    Value::Object(Map::from_iter([(
                        "input".to_string(),
                        Value::String(patch),
                    )])),
                ),
            ])))
        }
        "shell" => {
            let args: Vec<String> = if cmd.len() == 2 {
                split_command_line(cmd.get(1).and_then(|v| v.as_str()).unwrap_or(""))
                    .unwrap_or_default()
            } else {
                cmd.iter()
                    .skip(1)
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            };
            if args.is_empty() {
                return None;
            }
            Some(Value::Object(Map::from_iter([
                ("type".to_string(), Value::String("tool".to_string())),
                ("name".to_string(), Value::String("shell".to_string())),
                (
                    "input".to_string(),
                    Value::Object(Map::from_iter([(
                        "command".to_string(),
                        Value::Array(args.into_iter().map(Value::String).collect()),
                    )])),
                ),
            ])))
        }
        _ => None,
    }
}

/// Extract a single plausible shell command from free-form text.
///
/// This is intentionally conservative:
/// - Prefer a fenced code block (```...```), trim language tags, and accept
///   exactly one non-empty line.
/// - If no fence is present, fall back to a single inline backtick segment.
/// - Strip common prompt prefixes like `$ ` or `PS> `.
/// - Split the command line with a basic shell-aware splitter.
///
/// Returns the parsed argv vector when successful.
pub fn extract_single_shell_command(text: &str) -> Option<Vec<String>> {
    // 1) Prefer fenced block with a single command line
    if let Some(block) = strip_code_fence(text.trim()) {
        let mut lines = block.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
        let first = lines.next()?;
        // Reject multi-line blocks (likely scripts)
        if lines.next().is_some() {
            return None;
        }
        let cmd = strip_prompt_prefix(first);
        if cmd.is_empty() {
            return None;
        }
        return split_command_line(cmd);
    }

    // 2) Look for a single inline backtick-wrapped command
    if let Some((start, end)) = find_single_inline_backticks(text) {
        let cmd = text[start + 1..end].trim();
        let cmd = strip_prompt_prefix(cmd);
        if cmd.is_empty() {
            return None;
        }
        return split_command_line(cmd);
    }

    None
}

fn split_command_line(s: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_sq = false;
    let mut in_dq = false;
    let mut escaped = false;
    for ch in s.chars() {
        if escaped {
            cur.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_dq => {
                escaped = true;
            }
            '"' if !in_sq => {
                in_dq = !in_dq;
            }
            '\'' if !in_dq => {
                in_sq = !in_sq;
            }
            c if c.is_whitespace() && !in_sq && !in_dq => {
                if !cur.is_empty() {
                    args.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        args.push(cur);
    }
    Some(args)
}

fn normalize_possible_json_string(v: Value) -> Value {
    match v {
        Value::String(s) => {
            // If this string looks like JSON, try to parse it.
            let trimmed = s.trim();
            if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']')))
                && let Ok(val) = serde_json::from_str::<Value>(trimmed)
            {
                return val;
            }
            Value::String(s)
        }
        other => other,
    }
}

// Perform pragmatic repairs to common model-emitted JSON flaws
// - Remove comments
// - Convert single-quoted strings and keys to double-quoted
// - Quote unquoted keys
// - Remove trailing commas
// - Replace backticks / smart quotes
// - Attempt to balance braces at the end if clearly short by a small margin
pub fn repair_json(input: &str) -> String {
    let mut s = input.trim().to_string();

    // Strip code fences if present
    if let Some(stripped) = strip_code_fence(&s) {
        s = stripped.trim().to_string();
    }

    // Prefer a balanced object slice if we can find one
    if let Some(obj) = extract_balanced_object(&s) {
        s = obj;
    }

    s = remove_comments(&s);
    s = replace_smart_quotes(&s);
    // Some models emit invalid JSON escapes like \', which is not a valid
    // escape sequence in JSON. Inside double-quoted strings, turn \\' into '.
    s = fix_invalid_escaped_single_quotes(&s);
    s = convert_single_quoted_strings(&s);
    s = quote_unquoted_keys(&s);
    s = remove_trailing_commas(&s);
    s = balance_braces(&s);

    s
}

// Inside double-quoted JSON strings, a backslash before a single quote is invalid.
// Convert occurrences of \\' to ' while preserving other valid escapes.
fn fix_invalid_escaped_single_quotes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut in_dq = false; // in double-quoted string
    let mut esc = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_dq {
            if esc {
                // Previous was a backslash. If this is a single quote, drop the backslash.
                if ch == '\'' {
                    out.push('\'');
                    esc = false;
                    i += 1;
                    continue;
                }
                // Keep the backslash and current char for other escape sequences.
                out.push('\\');
                out.push(ch);
                esc = false;
                i += 1;
                continue;
            }
            match ch {
                '"' => {
                    in_dq = false;
                    out.push(ch);
                }
                '\\' => {
                    // Might be start of an escape sequence; handle on next iteration
                    esc = true;
                }
                _ => out.push(ch),
            }
            i += 1;
            continue;
        }
        if ch == '"' {
            in_dq = true;
            out.push(ch);
            i += 1;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    // If we ended while in an escape, we had a trailing backslash; keep it.
    if esc {
        out.push('\\');
    }
    out
}

fn remove_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut in_str = false;
    let mut esc = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_str {
            out.push(ch);
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if ch == '"' {
            in_str = true;
            out.push(ch);
            i += 1;
            continue;
        }
        if ch == '/' && i + 1 < bytes.len() {
            let next = bytes[i + 1] as char;
            if next == '/' {
                // line comment
                i += 2;
                while i < bytes.len() && (bytes[i] as char) != '\n' {
                    i += 1;
                }
                continue;
            } else if next == '*' {
                // block comment
                i += 2;
                while i + 1 < bytes.len()
                    && !((bytes[i] as char) == '*' && (bytes[i + 1] as char) == '/')
                {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
                continue;
            }
        }

        out.push(ch);
        i += 1;
    }
    out
}

fn replace_smart_quotes(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '“' | '”' | '‟' | '❝' | '❞' | '˝' => '"',
            '‘' | '’' | '‚' | '‛' | '❛' | '❜' => '\'',
            '`' => '"',
            _ => c,
        })
        .collect()
}

fn convert_single_quoted_strings(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut in_dq = false; // in double-quoted string
    let mut esc = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_dq {
            out.push(ch);
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_dq = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_dq = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == '\'' {
            // Start of a single-quoted string. Convert to double-quoted.
            i += 1;
            out.push('"');
            let mut esc_sq = false;
            while i < bytes.len() {
                let c = bytes[i] as char;
                if esc_sq {
                    // Preserve escape, but ensure quotes are properly escaped in JSON
                    out.push(match c {
                        '"' => '\\',
                        _ => '\\',
                    });
                    out.push(c);
                    esc_sq = false;
                } else if c == '\\' {
                    esc_sq = true;
                } else if c == '\'' {
                    // end of single-quoted string
                    out.push('"');
                    i += 1;
                    break;
                } else if c == '"' {
                    out.push_str("\\\"");
                } else {
                    out.push(c);
                }
                i += 1;
            }
            continue;
        }

        out.push(ch);
        i += 1;
    }
    out
}

fn quote_unquoted_keys(s: &str) -> String {
    // Simple state-machine pass: for sequences like { key: ... } or , key:
    // we insert quotes around key if not already quoted and it looks like an identifier.
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    let b = s.as_bytes();
    let mut in_str = false;
    let mut esc = false;
    while i < b.len() {
        let ch = b[i] as char;
        if in_str {
            out.push(ch);
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if ch == '"' {
            in_str = true;
            out.push(ch);
            i += 1;
            continue;
        }

        // Detect boundary that could precede a key
        if ch == '{' || ch == ',' {
            out.push(ch);
            i += 1;
            // consume whitespace
            while i < b.len() && (b[i] as char).is_whitespace() {
                out.push(b[i] as char);
                i += 1;
            }
            // attempt to read an identifier
            let start = i;
            if i < b.len() {
                let c = b[i] as char;
                if is_ident_start(c) {
                    i += 1;
                    while i < b.len() && is_ident_continue(b[i] as char) {
                        i += 1;
                    }
                    // peek for colon
                    let mut j = i;
                    while j < b.len() && (b[j] as char).is_whitespace() {
                        j += 1;
                    }
                    if j < b.len() && (b[j] as char) == ':' {
                        let key = &s[start..i];
                        out.push('"');
                        out.push_str(key);
                        out.push('"');
                        // copy whitespace and colon
                        while i < j {
                            out.push(b[i] as char);
                            i += 1;
                        }
                        out.push(':');
                        i += 1; // past colon
                        continue;
                    }
                }
            }
            // If not an identifier-colon pattern, just continue (we already wrote ch)
            continue;
        }

        out.push(ch);
        i += 1;
    }
    out
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '$'
}

fn remove_trailing_commas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0usize;
    let mut in_str = false;
    let mut esc = false;
    while i < b.len() {
        let ch = b[i] as char;
        if in_str {
            out.push(ch);
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if ch == '"' {
            in_str = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == ',' {
            // lookahead to see if the next non-ws is a closing ] or }
            let mut j = i + 1;
            while j < b.len() && (b[j] as char).is_whitespace() {
                j += 1;
            }
            if j < b.len() {
                let nxt = b[j] as char;
                if nxt == '}' || nxt == ']' {
                    // skip the comma
                    i += 1;
                    continue;
                }
            }
        }
        out.push(ch);
        i += 1;
    }
    out
}

fn balance_braces(s: &str) -> String {
    // Only attempt a gentle fix: if closing braces are fewer, append the missing ones.
    let mut in_str = false;
    let mut esc = false;
    let mut obj = 0i32;
    let mut arr = 0i32;
    for ch in s.chars() {
        if in_str {
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        if ch == '"' {
            in_str = true;
            continue;
        }
        match ch {
            '{' => obj += 1,
            '}' => obj -= 1,
            '[' => arr += 1,
            ']' => arr -= 1,
            _ => {}
        }
    }
    let mut out = s.to_string();
    if obj > 0 && obj <= 3 {
        for _ in 0..obj {
            out.push('}');
        }
    }
    if arr > 0 && arr <= 3 {
        for _ in 0..arr {
            out.push(']');
        }
    }
    out
}

fn strip_code_fence(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.starts_with(b"```") {
        // Skip optional language tag to end-of-line
        let after = &s[3..];
        let mut end_of_tag = 0usize;
        for (i, ch) in after.char_indices() {
            if ch == '\n' {
                end_of_tag = i + 1;
                break;
            }
        }
        let after_tag = &after[end_of_tag..];
        if let Some(end) = after_tag.rfind("```") {
            return Some(&after_tag[..end]);
        }
    }
    None
}

fn strip_prompt_prefix(s: &str) -> &str {
    let s = s.trim();
    for p in ["$ ", "PS> ", "> "] {
        if let Some(rest) = s.strip_prefix(p) {
            return rest.trim();
        }
    }
    s
}

fn find_single_inline_backticks(s: &str) -> Option<(usize, usize)> {
    let bytes = s.as_bytes();
    let mut start: Option<usize> = None;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            if let Some(st) = start {
                return Some((st, i));
            } else {
                start = Some(i);
            }
        }
        i += 1;
    }
    None
}

fn extract_balanced_object(s: &str) -> Option<String> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    let mut start_idx: Option<usize> = None;
    for (i, ch) in s.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start_idx = Some(i);
                }
                depth += 1;
            }
            '}' => {
                if depth > 0 {
                    depth -= 1;
                    if depth == 0 {
                        let start = start_idx?;
                        let end = i + ch.len_utf8();
                        return Some(s[start..end].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract a plausible apply_patch envelope from free-form text. Returns the
/// sanitized patch (e.g., stripping trailing comments after file paths) if a
/// `*** Begin Patch` ... `*** End Patch` block is found.
pub fn extract_patch_envelope(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut begin_idx = None;
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate() {
        let l = line.trim();
        if begin_idx.is_none() && l.to_ascii_lowercase().contains("begin patch") {
            begin_idx = Some(i);
        }
        if l.to_ascii_lowercase().contains("end patch") {
            end_idx = Some(i);
        }
    }
    let (start, end) = (begin_idx?, end_idx?);
    if end <= start {
        return None;
    }

    let mut cleaned = String::new();
    cleaned.push_str("*** Begin Patch\n");
    for &line in &lines[start + 1..end] {
        let line = line.trim_start();
        if let Some(h) = line.strip_prefix("*** Add File: ") {
            cleaned.push_str("*** Add File: ");
            cleaned.push_str(clean_file_target(h).as_str());
            cleaned.push('\n');
        } else if let Some(h) = line.strip_prefix("*** Update File: ") {
            cleaned.push_str("*** Update File: ");
            cleaned.push_str(clean_file_target(h).as_str());
            cleaned.push('\n');
        } else if let Some(h) = line.strip_prefix("*** Delete File: ") {
            cleaned.push_str("*** Delete File: ");
            cleaned.push_str(clean_file_target(h).as_str());
            cleaned.push('\n');
        } else if let Some(h) = line.strip_prefix("*** Move to: ") {
            cleaned.push_str("*** Move to: ");
            cleaned.push_str(clean_file_target(h).as_str());
            cleaned.push('\n');
        } else {
            cleaned.push_str(line);
            cleaned.push('\n');
        }
    }
    cleaned.push_str("*** End Patch\n");
    Some(cleaned)
}

fn clean_file_target(rest: &str) -> String {
    let r = rest.trim();
    // stop at common comment separators
    for sep in [" -- ", "  -- ", "\t-- ", " # ", " (", ") "] {
        if let Some(idx) = r.find(sep) {
            return r[..idx].trim().to_string();
        }
    }
    r.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn extracts_single_command_from_fence() {
        let text = "```sh\npython game.py --help\n```";
        let argv = extract_single_shell_command(text).expect("argv");
        assert_eq!(argv, vec!["python", "game.py", "--help"]);
    }

    #[test]
    fn extracts_single_command_from_inline_backticks() {
        let text = "Run `python -m venv .venv` then continue";
        let argv = extract_single_shell_command(text).expect("argv");
        assert_eq!(argv, vec!["python", "-m", "venv", ".venv"]);
    }

    #[test]
    fn strips_prompt_prefixes() {
        for prefix in ["$ ", "PS> ", "> "] {
            let text = format!("```\n{}echo hi\n```", prefix);
            let argv = extract_single_shell_command(&text).expect("argv");
            assert_eq!(argv, vec!["echo", "hi"]);
        }
    }
}
