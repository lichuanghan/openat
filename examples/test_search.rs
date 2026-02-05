use reqwest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearch {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tools {
    pub web_search: WebSearch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub tools: Tools,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    title: String,
    url: String,
    description: String,
}

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

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".nanobot")
        .join("config.json")
}

fn load_config() -> Option<Config> {
    let path = config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return Some(config);
            }
        }
    }
    None
}

async fn search_brave(api_key: &str, query: &str) -> Result<Vec<SearchResult>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.search.brave.com/v1/web/search?q={}",
        urlencoding::encode(query)
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("X-Subscription-Token", api_key)
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

#[tokio::main]
async fn main() {
    println!("Testing Brave Search API...\n");

    // Try to load API key from config
    let api_key = if let Some(config) = load_config() {
        if !config.tools.web_search.api_key.is_empty() {
            println!("Loaded API key from config");
            config.tools.web_search.api_key
        } else {
            println!("No API key found in config (~/.nanobot/config.json)");
            println!("Please add your Brave API key to the config:");
            println!(r#"{{"tools": {{"web_search": {{"api_key": "YOUR_API_KEY"}}}}}}"#);
            return;
        }
    } else {
        println!("Config file not found at ~/.nanobot/config.json");
        println!("Please create it with your Brave API key:");
        println!(r#"{{"tools": {{"web_search": {{"api_key": "YOUR_API_KEY"}}}}}}"#);
        return;
    };

    match search_brave(&api_key, "Rust programming language").await {
        Ok(results) => {
            println!("\nSearch results for 'Rust programming language':\n");
            for (i, result) in results.iter().take(5).enumerate() {
                println!("{}. {}", i + 1, result.title);
                println!("   URL: {}", result.url);
                println!("   {}\n", result.description);
            }
            println!("Total results: {}", results.len());
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
