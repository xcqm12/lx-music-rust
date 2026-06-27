//! HTTP Utilities Module
//! 
//! Provides HTTP client functionality for fetching music URLs,
//! lyrics, and album artwork from various music sources.

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// HTTP client singleton
static HTTP_CLIENT: Lazy<Arc<Mutex<Option<Client>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

/// Initialize HTTP client
pub fn init_client() -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;
    
    let mut guard = HTTP_CLIENT.lock().map_err(|e| e.to_string())?;
    *guard = Some(client);
    Ok(())
}

/// Get HTTP client
fn get_client() -> Result<Client, String> {
    let guard = HTTP_CLIENT.lock().map_err(|e| e.to_string())?;
    guard.clone().ok_or_else(|| "HTTP client not initialized".to_string())
}

/// HTTP response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

/// GET request
pub fn get(url: &str) -> Result<HttpResponse, String> {
    let client = get_client()?;
    
    let response = client.get(url)
        .send()
        .map_err(|e| e.to_string())?;
    
    let status = response.status().as_u16();
    
    // Extract headers before consuming response
    let headers: HashMap<String, String> = response.headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    
    let body = response.text().map_err(|e| e.to_string())?;
    
    Ok(HttpResponse { status, body, headers })
}

/// POST request with JSON body
pub fn post_json(url: &str, body: &str) -> Result<HttpResponse, String> {
    let client = get_client()?;
    
    let response = client.post(url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| e.to_string())?;
    
    let status = response.status().as_u16();
    
    // Extract headers before consuming response
    let headers: HashMap<String, String> = response.headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    
    let resp_body = response.text().map_err(|e| e.to_string())?;
    
    Ok(HttpResponse { status, body: resp_body, headers })
}

/// HTTP Client wrapper for async operations
pub struct HttpClient;

impl HttpClient {
    /// Fetch music URL from source
    pub fn fetch_music_url(url: &str) -> Result<String, String> {
        let response = get(url)?;
        if response.status == 200 {
            Ok(response.body)
        } else {
            Err(format!("HTTP error: {}", response.status))
        }
    }

    /// Fetch lyric content
    pub fn fetch_lyric(url: &str) -> Result<String, String> {
        let response = get(url)?;
        if response.status == 200 {
            Ok(response.body)
        } else {
            Err(format!("HTTP error: {}", response.status))
        }
    }

    /// Fetch image as base64
    pub fn fetch_image_as_base64(url: &str) -> Result<String, String> {
        let client = get_client()?;
        
        let response = client.get(url)
            .send()
            .map_err(|e| e.to_string())?;
        
        if response.status().as_u16() != 200 {
            return Err(format!("HTTP error: {}", response.status().as_u16()));
        }
        
        let bytes = response.bytes().map_err(|e| e.to_string())?;
        Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes))
    }

    /// Download file to bytes
    pub fn download_to_bytes(url: &str) -> Result<Vec<u8>, String> {
        let client = get_client()?;
        
        let response = client.get(url)
            .send()
            .map_err(|e| e.to_string())?;
        
        if response.status().as_u16() != 200 {
            return Err(format!("HTTP error: {}", response.status().as_u16()));
        }
        
        response.bytes().map(|b| b.to_vec()).map_err(|e| e.to_string())
    }
}