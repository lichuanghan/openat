//! HTML utilities - shared HTML parsing and text extraction.
//!
//! Provides functions for stripping HTML tags, extracting titles,
//! and converting HTML to text/markdown.

use regex::Regex;

/// Strip all HTML tags from content
pub fn strip_tags(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    let mut script_style = false;
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            // Collect the tag name to check for script/style
            let mut tag_chars = Vec::new();
            for pc in chars.by_ref() {
                if pc == '>' {
                    break;
                }
                tag_chars.push(pc);
            }
            let tag_str: String = tag_chars.iter().collect();
            let tag_lower = tag_str.to_lowercase();
            if tag_lower.contains("<script") || tag_lower.starts_with("script")
                || tag_lower.contains("<style") || tag_lower.starts_with("style") {
                script_style = true;
            }
            in_tag = false;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag && !script_style {
            text.push(c);
        } else if script_style {
            // Check if we're exiting script/style tag
            if c == '<' {
                // Look ahead to see if this closes the tag
                let mut tag_chars = Vec::new();
                for pc in chars.by_ref() {
                    if pc == '>' {
                        break;
                    }
                    tag_chars.push(pc);
                }
                let tag_str: String = tag_chars.iter().collect();
                let tag_lower = tag_str.to_lowercase();
                if tag_lower.starts_with("/script") || tag_lower.starts_with("/style") {
                    script_style = false;
                }
            }
        }
    }

    // Normalize whitespace
    let re = Regex::new(r"[ \t]+").unwrap();
    text = re.replace_all(&text, " ").to_string();
    let re = Regex::new(r"\n{3,}").unwrap();
    text = re.replace_all(&text, "\n\n").to_string();

    // Decode common HTML entities
    text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");

    text.trim().to_string()
}

/// Extract title from HTML
pub fn extract_title(html: &str) -> Option<String> {
    let title_pattern = Regex::new(r"<title[^>]*>([^<]+)</title>").ok()?;
    title_pattern.captures(html).and_then(|cap| {
        cap.get(1).map(|m| {
            strip_tags(m.as_str().trim())
                .replace("&nbsp;", " ")
                .replace("&amp;", "&")
        })
    })
}

/// Check if content looks like HTML
pub fn is_html(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.to_lowercase().starts_with("<!doctype") || trimmed.starts_with("<html") || trimmed.starts_with("<head")
}

/// Convert HTML to markdown-like format
pub fn convert_to_markdown(content: &str, title: &str) -> String {
    let mut text = content.to_string();

    // Convert headings (simple h1-h6 patterns)
    let heading_pattern = Regex::new(r"(?m)^#{1,6}\s+(.+)$").unwrap();
    text = heading_pattern.replace_all(&text, |caps: &regex::Captures| {
        format!("## {}", caps.get(1).map_or("", |m| m.as_str()))
    }).to_string();

    // Convert bullet lists
    let list_pattern = Regex::new(r"(?m)^[\*\-\+]\s+(.+)$").unwrap();
    text = list_pattern.replace_all(&text, "- $1").to_string();

    // Convert numbered lists
    let num_pattern = Regex::new(r"(?m)^\d+\.\s+(.+)$").unwrap();
    text = num_pattern.replace_all(&text, "1. $1").to_string();

    // Convert links [text](url)
    let link_pattern = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
    text = link_pattern.replace_all(&text, "$1: $2").to_string();

    text
}

/// Simple HTML to text extraction (for quick extraction)
pub fn extract_text(html: &str) -> String {
    strip_tags(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tags_basic() {
        let html = "<p>Hello <b>World</b></p>";
        assert_eq!(strip_tags(html), "Hello World");
    }

    #[test]
    fn test_strip_tags_script_style() {
        let html = "<p>Visible</p><script>Hidden</script><style>Also Hidden</style>";
        assert_eq!(strip_tags(html), "Visible");
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>Test Page</title></head></html>";
        assert_eq!(extract_title(html), Some("Test Page".to_string()));
    }

    #[test]
    fn test_is_html() {
        assert!(is_html("<html>test</html>"));
        assert!(is_html("<!DOCTYPE html>test"));
        assert!(!is_html("plain text"));
    }

    #[test]
    fn test_convert_to_markdown() {
        let html = "# Heading\n* item 1\n* item 2";
        let result = convert_to_markdown(html, "");
        assert!(result.contains("## Heading"));
        assert!(result.contains("- item 1"));
    }

    #[test]
    fn test_extract_text() {
        let html = "<div>Simple text</div>";
        assert_eq!(extract_text(html), "Simple text");
    }
}
