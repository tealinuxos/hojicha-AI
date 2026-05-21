pub trait ColorExt {
    fn green_or_colored(&self) -> String;
    fn yellow_or_colored(&self) -> String;
    fn dimmed_colored(&self) -> String;
}

impl ColorExt for str {
    fn green_or_colored(&self) -> String {
        use colored::Colorize;
        self.truecolor(134, 239, 172).bold().to_string()
    }
    fn yellow_or_colored(&self) -> String {
        use colored::Colorize;
        self.truecolor(251, 191, 36).bold().to_string()
    }
    fn dimmed_colored(&self) -> String {
        use colored::Colorize;
        self.truecolor(160, 160, 160).to_string()
    }
}

pub fn extract_json(text: &str) -> String {
    let stripped = if let Some(think_end) = text.find("</think>") {
        text[think_end + 8..].trim()
    } else {
        text.trim()
    };

    let mut raw_json = if let Some(start) = stripped.find("```json") {
        if let Some(end) = stripped[start + 7..].find("```") {
            stripped[start + 7..start + 7 + end].trim().to_string()
        } else {
            stripped[start + 7..].trim().to_string()
        }
    } else if let Some(start) = stripped.find("```") {
        if let Some(end) = stripped[start + 3..].find("```") {
            stripped[start + 3..start + 3 + end].trim().to_string()
        } else {
            stripped[start + 3..].trim().to_string()
        }
    } else {
        stripped.to_string()
    };

    if let Some(start) = raw_json.find('{') {
        if let Some(end) = raw_json.rfind('}') {
            if end > start {
                raw_json = raw_json[start..=end].to_string();
            }
        }
    }

    clean_json_newlines(&raw_json)
}

pub fn clean_json_newlines(json_str: &str) -> String {
    let mut in_quotes = false;
    let mut escaped = false;
    let mut result = String::new();

    for c in json_str.chars() {
        if c == '\\' {
            escaped = !escaped;
            result.push(c);
        } else if c == '"' {
            if !escaped {
                in_quotes = !in_quotes;
            }
            escaped = false;
            result.push(c);
        } else {
            escaped = false;
            if c == '\n' || c == '\r' {
                if in_quotes {
                    result.push_str("\\n");
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }
    }
    result
}

pub fn clean_raw_command(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(start) = trimmed.find("\"command\":") {
        let rest = &trimmed[start + 10..];
        if let Some(val_start) = rest.find('"') {
            let val_rest = &rest[val_start + 1..];
            if let Some(val_end) = val_rest.find('"') {
                return val_rest[..val_end].to_string();
            }
        }
    }
    for line in trimmed.lines() {
        let line_trimmed = line.trim();
        if !line_trimmed.is_empty() {
            let mut cleaned = line_trimmed.to_string();
            if cleaned.starts_with('"') && cleaned.ends_with('"') {
                cleaned = cleaned[1..cleaned.len() - 1].to_string();
            }
            if cleaned.starts_with('\'') && cleaned.ends_with('\'') {
                cleaned = cleaned[1..cleaned.len() - 1].to_string();
            }
            if cleaned.starts_with('`') && cleaned.ends_with('`') {
                cleaned = cleaned[1..cleaned.len() - 1].to_string();
            }
            return cleaned;
        }
    }
    trimmed.to_string()
}
