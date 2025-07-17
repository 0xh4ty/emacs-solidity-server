use lsp_types::Position;

/// Convert byte offset to LSP position (line + column)
pub fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut col = 0;
    let mut current_offset = 0;

    for l in source.lines() {
        let line_len = l.len() + 1; // account for newline
        if current_offset + line_len > offset {
            col = offset - current_offset;
            break;
        }
        current_offset += line_len;
        line += 1;
    }

    Position::new(line as u32, col as u32)
}

/// Convert LSP position to byte offset in file
pub fn position_to_byte_offset(source: &str, pos: Position) -> Option<usize> {
    let mut offset = 0;
    let mut lines = source.lines();

    for _ in 0..pos.line {
        offset += lines.next()?.len() + 1; // +1 for newline
    }

    let target_line = lines.next()?;
    let char_offset = pos.character as usize;

    if char_offset > target_line.len() {
        return None; // out of bounds
    }

    Some(offset + char_offset)
}
