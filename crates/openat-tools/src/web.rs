//! Web tools (search, fetch)

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use crate::{Tool, ToolDefinition};

/// Web search tool
#[derive(Debug, Clone)]
pub struct WebTool {
    api_key: String,
}

impl WebTool {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Tool for WebTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "web_search",
            "Search the web for information.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" }
                },
                "required": ["query"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            query: String,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        if self.api_key.is_empty() {
            return Err("Brave API key not configured".to_string());
        }

        let client = reqwest::Client::new();
        let url = format!(
            "https://api.search.brave.com/v1/web/search?q={}",
            urlencoding::encode(&args.query)
        );

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("Search failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Search failed with status: {}", response.status()));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        let results = json.get("web").and_then(|w| w.get("results"))
            .and_then(|r| r.as_array())
            .ok_or("No results")?;

        let mut output = format!("Search results for '{}':\n\n", args.query);

        for (i, result) in results.iter().take(5).enumerate() {
            let title = result.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let url = result.get("url").and_then(|t| t.as_str()).unwrap_or("");
            let desc = result.get("description").and_then(|t| t.as_str()).unwrap_or("");

            output += &format!("{}. {}\n   URL: {}\n   {}\n\n", i + 1, title, url, desc);
        }

        Ok(output)
    }
}
