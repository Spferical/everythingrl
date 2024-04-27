pub fn trim(text: &str, max_chars: usize) -> &str {
    if text.chars().next().is_none() {
        return "";
    }
    let length = text.len().min(max_chars);
    let mut iter = text.char_indices();
    let (end, _) = iter
        .nth(length)
        .unwrap_or(text.char_indices().last().unwrap());
    &text[..end]
}
