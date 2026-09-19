pub fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('=') {
        return None;
    }
    let mut level: u8 = 0;
    for c in trimmed.chars() {
        if c == '=' {
            level += 1;
        } else {
            break;
        }
    }
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level as usize..];
    if rest.is_empty() || !rest.starts_with(' ') {
        return None;
    }
    Some((level, rest))
}

pub fn is_heading(line: &str) -> bool {
    parse_heading(line).is_some()
}
