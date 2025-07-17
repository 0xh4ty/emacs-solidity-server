pub fn extract_identifier_at(source: &str, offset: usize) -> Option<String> {
    let bytes = source.as_bytes();

    if offset >= bytes.len() {
        return None;
    }

    let is_ident_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_';

    let mut start = offset;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = offset;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }

    if start < end {
        Some(source[start..end].to_string())
    } else {
        None
    }
}
