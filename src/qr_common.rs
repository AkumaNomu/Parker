pub fn is_safe_web_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() > 4096
        || trimmed
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

#[cfg(test)]
mod tests {
    use super::is_safe_web_url;

    #[test]
    fn accepts_only_safe_http_urls() {
        assert!(is_safe_web_url("https://example.com/path"));
        assert!(is_safe_web_url("HTTP://localhost:3000"));
        assert!(!is_safe_web_url("javascript:alert(1)"));
        assert!(!is_safe_web_url("https://example.com/a b"));
    }
}
