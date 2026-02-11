





//! Web tools - web search and fetch.

use crate::config::Config;
use reqwest;
use serde::{Deserialize, Serialize};

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
    /// Create a new Brave Search client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Create from config
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
        let text = crate::tools::html::extract_text(&html);
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
                        output += &format!(
                            "{}. {}\n   URL: {}\n   {}\n\n",
                            i + 1, result.title, result.url, result.description
                        );
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
                }
                .to_string();
                format!("Content from {}:\n\n{}\n\n( truncated to 2000 chars )", url, truncated)
            }
            Err(e) => format!("Fetch error: {}", e),
        }
    } else {
        "Web fetch not configured.".to_string()
    }
}
