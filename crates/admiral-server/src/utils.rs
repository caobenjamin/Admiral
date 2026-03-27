//! 通用工具：UTF-8 安全截断、日志预览等。

/// 在不超过 `max_bytes` 字节的最后一个 UTF-8 字符边界处截断，避免 `s[..max_bytes]` 在多字节字符中间导致 panic。
#[inline]
pub fn truncate_utf8_at(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Preview bytes for logging: converts to string and truncates.
pub fn bytes_preview(bytes: &[u8], limit: usize) -> String {
    if bytes.is_empty() {
        return "(empty)".to_string();
    }
    match String::from_utf8(bytes.to_vec()) {
        Ok(s) => {
            if s.len() > limit {
                format!(
                    "{}... (truncated, {} chars)",
                    truncate_utf8_at(&s, limit),
                    s.len()
                )
            } else {
                s
            }
        }
        Err(_) => format!("<binary, {} bytes>", bytes.len()),
    }
}

/// Truncate string for database storage.
pub fn truncate_for_db(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}... (truncated)", truncate_utf8_at(s, max))
    }
}
