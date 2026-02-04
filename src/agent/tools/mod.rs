use crate::config::Config;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

/// Web search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub description: String,
}

/// Brave Search API client
#[derive(Debug, Clone)]
pub struct BraveSearch {
    api_key: String,
    client: reqwest::Client,
}

impl BraveSearch {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_config(config: &Config) -> Option<Self> {
        let api_key = &config.tools.web_search.api_key;
        if api_key.is_empty() {
            None
        } else {
            Some(Self::new(api_key.clone()))
        }
    }

    /// Search the web
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        let url = format!(
            "https://api.search.brave.com/v1/web/search?q={}",
            urlencoding::encode(query)
        );

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("Search request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Search API error: {}", error));
        }

        let response_json: BraveResponse = response
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        Ok(response_json
            .results
            .into_iter()
            .map(|r| SearchResult {
                title: r.title,
                url: r.url,
                description: r.description,
            })
            .collect())
    }

    /// Fetch URL content
    pub async fn fetch(&self, url: &str) -> Result<String, String> {
        let response = self
            .client
            .get(url)
            .header("Accept", "text/html")
            .send()
            .await
            .map_err(|e| format!("Fetch request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Fetch failed with status: {}", response.status()));
        }

        let html = response
            .text()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        // Simple HTML to text extraction
        let text = extract_text(&html);
        Ok(text)
    }
}

/// Brave API response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BraveResponse {
    results: Vec<BraveResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BraveResult {
    title: String,
    url: String,
    description: String,
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

    // Clean up whitespace
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Get web search tool definition for LLM
pub fn get_web_search_tool_definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "web_search",
            "description": "Search the web for information. Use this when you need current events or information not in your training data.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }
        }
    })
}

/// Get web fetch tool definition for LLM
pub fn get_web_fetch_tool_definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "web_fetch",
            "description": "Fetch and extract text content from a URL.",
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch"
                    }
                },
                "required": ["url"]
            }
        }
    })
}

/// Execute web search
pub async fn execute_web_search(config: &Config, query: &str) -> String {
    if let Some(search) = BraveSearch::from_config(config) {
        match search.search(query).await {
            Ok(results) => {
                if results.is_empty() {
                    "No results found.".to_string()
                } else {
                    let mut output = format!("Search results for '{}':\n\n", query);
                    for (i, result) in results.iter().take(5).enumerate() {
                        output += &format!("{}. {}\n   URL: {}\n   {}\n\n",
                            i + 1, result.title, result.url, result.description);
                    }
                    output
                }
            }
            Err(e) => format!("Search error: {}", e),
        }
    } else {
        "Web search not configured. Add Brave API key to config.".to_string()
    }
}

/// Execute web fetch
pub async fn execute_web_fetch(config: &Config, url: &str) -> String {
    if let Some(search) = BraveSearch::from_config(config) {
        match search.fetch(url).await {
            Ok(content) => {
                let truncated = if content.len() > 2000 {
                    &content[..2000]
                } else {
                    &content
                }.to_string();
                format!("Content from {}:\n\n{}\n\n( truncated to 2000 chars )", url, truncated)
            }
            Err(e) => format!("Fetch error: {}", e),
        }
    } else {
        "Web fetch not configured.".to_string()
    }
}
