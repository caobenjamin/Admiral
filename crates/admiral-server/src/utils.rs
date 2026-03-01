//! 通用工具：UTF-8 安全截断等。

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
