//! Web Fetch Tool - Fetch and extract text content from URLs.
//!
//! Uses HTML parsing for content extraction.

use crate::config::Config;
use reqwest;
use std::time::Duration;

/// Extract mode for web fetch
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtractMode {
    Markdown,
    Text,
}

/// Fetch result
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub url: String,
    pub final_url: String,
    pub status: u16,
    pub extractor: String,
    pub truncated: bool,
    pub length: usize,
    pub text: String,
}

/// Fetch URL content and extract text
pub async fn execute_web_fetch(
    _config: &Config,
    url: &str,
    extract_mode: ExtractMode,
    max_chars: usize,
) -> Result<FetchResult, String> {
    // Validate URL
    let (is_valid, error_msg) = validate_url(url);
    if !is_valid {
        return Err(format!("URL validation failed: {}", error_msg));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let response = match client.get(url).send().await {
        Ok(resp) => resp,
        Err(e) => return Err(format!("Fetch error: {}", e)),
    };

    let final_url = response.url().to_string();
    let status = response.status().as_u16();

    if !response.status().is_success() {
        return Err(format!("Fetch failed with status: {}", status));
    }

    // Clone content-type header before consuming response
    let content_type: String = response
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();

    let html = match response.text().await {
        Ok(text) => text,
        Err(e) => return Err(format!("Parse error: {}", e)),
    };

    // Extract content based on content type
    let (text, extractor) = if content_type.contains("application/json") {
        let json_text = format_json(&html);
        (json_text, "json".to_string())
    } else if is_html(&html) {
        let title = extract_title(&html);
        let content = strip_tags(&html);

        let title_str = title.as_deref().unwrap_or("");
        let processed = if extract_mode == ExtractMode::Markdown {
            convert_to_markdown(&content, title_str)
        } else {
            content
        };

        let full_text = if let Some(t) = &title {
            format!("# {}\n\n{}", t, processed)
        } else {
            processed
        };

        (full_text, "html".to_string())
    } else {
        (html, "raw".to_string())
    };

    let truncated = text.len() > max_chars;
    let result_text = if truncated {
        text[..max_chars].to_string()
    } else {
        text
    };

    Ok(FetchResult {
        url: url.to_string(),
        final_url,
        status,
        extractor,
        truncated,
        length: result_text.len(),
        text: result_text,
    })
}

/// Validate URL - only http/https allowed
fn validate_url(url: &str) -> (bool, String) {
    if url.is_empty() {
        return (false, "URL is empty".to_string());
    }

    match url::Url::parse(url) {
        Ok(p) => {
            if p.scheme() != "http" && p.scheme() != "https" {
                (false, format!("Only http/https allowed, got '{}'", p.scheme()))
            } else if p.host().is_none() {
                (false, "Missing domain".to_string())
            } else {
                (true, String::new())
            }
        }
        Err(e) => (false, format!("Invalid URL: {}", e)),
    }
}

/// Check if content looks like HTML
fn is_html(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.starts_with("<!doctype") || trimmed.starts_with("<html") || trimmed.starts_with("<head")
}

/// Extract title from HTML
fn extract_title(html: &str) -> Option<String> {
    let title_pattern = regex::Regex::new(r"<title[^>]*>([^<]+)</title>").ok()?;
    title_pattern.captures(html).and_then(|cap| {
        cap.get(1).map(|m| {
            strip_tags(m.as_str().trim())
                .replace("&nbsp;", " ")
                .replace("&amp;", "&")
        })
    })
}

/// Format JSON for display
fn format_json(json: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or(json.to_string()),
        Err(_) => json.to_string(),
    }
}

/// Convert content to markdown-like format
fn convert_to_markdown(content: &str, title: &str) -> String {
    let mut text = content.to_string();

    // Convert headings (simple h1-h6 patterns)
    let heading_pattern = regex::Regex::new(r"(?m)^#{1,6}\s+(.+)$").unwrap();
    text = heading_pattern.replace_all(&text, |caps: &regex::Captures| {
        format!("## {}", caps.get(1).map_or("", |m| m.as_str()))
    }).to_string();

    // Convert bullet lists
    let list_pattern = regex::Regex::new(r"(?m)^[\*\-\+]\s+(.+)$").unwrap();
    text = list_pattern.replace_all(&text, "- $1").to_string();

    // Convert numbered lists
    let num_pattern = regex::Regex::new(r"(?m)^\d+\.\s+(.+)$").unwrap();
    text = num_pattern.replace_all(&text, "1. $1").to_string();

    // Convert links [text](url)
    let link_pattern = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
    text = link_pattern.replace_all(&text, "$1: $2").to_string();

    text
}

/// Strip HTML tags
fn strip_tags(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    let mut script_style = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
            // Check if we're entering script or style tag
            let lower = html[html.len().saturating_sub(20)..]
                .to_lowercase();
            if lower.contains("<script") || lower.contains("<style") {
                script_style = true;
            }
        } else if c == '>' {
            in_tag = false;
            if script_style && (c == 's' || c == 'S' || c == '/' || c == 't' || c == 'T') {
                // Check if we're exiting script or style tag
                let recent: String = text.chars().rev().take(20).collect();
                if recent.contains("script") || recent.contains("style") {
                    script_style = false;
                }
            }
        } else if !in_tag && !script_style {
            text.push(c);
        }
    }

    // Normalize whitespace
    let re = regex::Regex::new(r"[ \t]+").unwrap();
    text = re.replace_all(&text, " ").to_string();
    let re = regex::Regex::new(r"\n{3,}").unwrap();
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
