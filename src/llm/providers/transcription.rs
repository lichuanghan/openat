//! Groq Whisper transcription provider.
//!
//! Uses Groq's Whisper API for fast audio transcription.

use std::path::Path;
use tracing::{debug, error, warn};

/// Groq Whisper transcription provider
#[derive(Debug, Clone)]
pub struct GroqTranscriptionProvider {
    api_key: String,
    api_url: String,
    model: String,
}

impl GroqTranscriptionProvider {
    /// Create a new transcription provider
    ///
    /// Uses GROQ_API_KEY from environment if not provided.
    pub fn new(api_key: Option<String>) -> Self {
        let api_key = api_key.or_else(|| std::env::var("GROQ_API_KEY").ok())
            .unwrap_or_default();

        Self {
            api_key,
            api_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3".to_string(),
        }
    }

    /// Check if the provider is configured
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Transcribe an audio file
    ///
    /// Args:
    ///     file_path: Path to the audio file
    ///
    /// Returns:
    ///     Transcribed text, or error message if transcription fails
    pub async fn transcribe(&self, file_path: &Path) -> Result<String, String> {
        if !self.is_configured() {
            warn!("Groq API key not configured for transcription");
            return Err("Groq API key not configured".to_string());
        }

        if !file_path.exists() {
            error!("Audio file not found: {:?}", file_path);
            return Err(format!("Audio file not found: {:?}", file_path));
        }

        debug!("Transcribing audio file: {:?}", file_path);

        // Read file content
        let file_content = tokio::fs::read(file_path)
            .await
            .map_err(|e| format!("Failed to read audio file: {}", e))?;

        // Get file name with owned string to avoid lifetime issues
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "audio.wav".to_string());

        // Create multipart form
        let file_part = reqwest::multipart::Part::bytes(file_content)
            .file_name(file_name)
            .mime_str("audio/wav")
            .map_err(|e| format!("Failed to create multipart part: {}", e))?;

        let form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone());

        let client = reqwest::Client::new();
        let response = client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Transcription API error: {} - {}", status, body);
            return Err(format!("API error {}: {}", status, body));
        }

        #[derive(serde::Deserialize)]
        struct Response {
            text: String,
        }

        let response_data: Response = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        debug!("Transcription complete: {} chars", response_data.text.len());
        Ok(response_data.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_provider_default() {
        // Test provider creation with API key passed directly
        let provider = GroqTranscriptionProvider::new(Some("test-key".to_string()));
        assert!(provider.is_configured());
        assert_eq!(provider.api_url, "https://api.groq.com/openai/v1/audio/transcriptions");
        assert_eq!(provider.model, "whisper-large-v3");
    }

    #[test]
    fn test_transcription_provider_not_configured() {
        // Test without API key
        let provider = GroqTranscriptionProvider::new(None);
        assert!(!provider.is_configured());
    }

    #[test]
    fn test_transcribe_nonexistent_file() {
        let provider = GroqTranscriptionProvider::new(Some("test-key".to_string()));
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(provider.transcribe(Path::new("/nonexistent/file.wav")));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_transcribe_provider_creation() {
        let provider = GroqTranscriptionProvider::new(Some("test-key".to_string()));
        assert!(provider.is_configured());
        assert_eq!(provider.api_key, "test-key");
    }
}
