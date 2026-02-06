//! Web Fetch Tool - Fetch and extract text content from URLs.

use crate::config::Config;
use reqwest;

/// Fetch URL content and extract text
pub async fn execute_web_fetch(config: &Config, url: &str) -> String {
    let client = reqwest::Client::new();

    let response = match client.get(url).send().await {
        Ok(resp) => resp,
        Err(e) => return format!("Fetch error: {}", e),
    };

    if !response.status().is_success() {
        return format!("Fetch failed with status: {}", response.status());
    }

    let html = match response.text().await {
        Ok(text) => text,
        Err(e) => return format!("Parse error: {}", e),
    };

    // Simple HTML to text extraction
    let text = extract_text(&html);
    let truncated = if text.len() > 2000 {
        &text[..2000]
    } else {
        &text
    }
    .to_string();

    format!("Content from {}:\n\n{}\n\n( truncated to 2000 chars )", url, truncated)
}

/// Simple HTML to text extraction
fn extract_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            text.push(c);
        }
    }

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
