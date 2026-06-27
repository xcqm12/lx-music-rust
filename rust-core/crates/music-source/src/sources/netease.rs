use crate::{MusicSourceProvider, crypto, utils};
use common::{MusicInfo, MusicQuality, MusicSource, LyricInfo, Result, Error};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

pub struct NeteaseSource {
    client: Arc<Client>,
    base_url: String,
}

impl NeteaseSource {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            base_url: "https://music.163.com".to_string(),
        }
    }
    
    /// 加密请求数据 (weapi)
    fn encrypt_data(&self, data: &str) -> Result<Value> {
        let text = data;
        let secret_key = b"0CoJUm6Qyw8W8jud";
        let iv = b"0102030405060708";
        
        // 第一次 AES 加密
        let first = crypto::aes_cbc_encrypt(text.as_bytes(), secret_key, iv);
        
        // 生成随机 key
        let rand_key: String = (0..16)
            .map(|_| (b'a' + rand::random::<u8>() % 26) as char)
            .collect();
        
        // 第二次 AES 加密
        let second = crypto::aes_cbc_encrypt(&first, rand_key.as_bytes(), iv);
        
        // RSA 加密随机 key
        let rsa_key = "010001";
        let rsa_modulus = "00e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7";
        let encrypted_key = crypto::rsa_encrypt(&rand_key, rsa_key, rsa_modulus);
        
        Ok(serde_json::json!({
            "params": crypto::base64_encode(&second),
            "encSecKey": encrypted_key,
        }))
    }
    
    /// 构建请求头
    fn build_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Referer", "https://music.163.com".to_string()),
            ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string()),
            ("Content-Type", "application/x-www-form-urlencoded".to_string()),
        ]
    }
    
    /// 发送 weapi 请求
    async fn weapi_request(&self, endpoint: &str, data: Value) -> Result<Value> {
        let url = format!("{}/weapi/{}", self.base_url, endpoint);
        
        let encrypted = self.encrypt_data(&data.to_string())?;
        
        let resp = self.client
            .post(&url)
            .headers(self.build_headers().into_iter().map(|(k, v)| {
                (k.to_string(), v)
            }).collect::<std::collections::HashMap<_, _>>())
            .form(&encrypted)
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        Ok(json)
    }
}

#[async_trait::async_trait]
impl MusicSourceProvider for NeteaseSource {
    fn name(&self) -> &str {
        "网易云音乐"
    }
    
    fn source_id(&self) -> MusicSource {
        MusicSource::Wy
    }
    
    async fn search(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> Result<Vec<MusicInfo>> {
        let data = serde_json::json!({
            "s": keyword,
            "type": 1,
            "offset": (page - 1) * limit,
            "limit": limit,
        });
        
        let json = self.weapi_request("cloudsearch/get/web", data).await?;
        
        let mut results = Vec::new();
        
        if let Some(songs) = json.get("result").and_then(|r| r.get("songs")).and_then(|s| s.as_array()) {
            for song in songs {
                if let Ok(music_info) = self.parse_music_info(song) {
                    results.push(music_info);
                }
            }
        }
        
        Ok(results)
    }
    
    async fn get_music_url(
        &self,
        music_info: &MusicInfo,
        quality: MusicQuality,
    ) -> Result<String> {
        let br = match quality {
            MusicQuality::Lq => 128000,
            MusicQuality::Mq => 192000,
            MusicQuality::Hq => 320000,
            MusicQuality::Sq => 999000,
            MusicQuality::Hires => 999000,
        };
        
        let data = serde_json::json!({
            "ids": [music_info.id.parse::<u64>().unwrap_or(0)],
            "br": br,
        });
        
        let json = self.weapi_request("song/enhance/player/url", data).await?;
        
        if let Some(urls) = json.get("data").and_then(|d| d.as_array()) {
            if let Some(first) = urls.first() {
                if let Some(url) = first.get("url").and_then(|u| u.as_str()) {
                    return Ok(url.to_string());
                }
            }
        }
        
        Err(Error::MusicSource("Failed to get music URL".to_string()))
    }
    
    async fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let data = serde_json::json!({
            "id": music_info.id.parse::<u64>().unwrap_or(0),
            "lv": -1,
            "tv": -1,
        });
        
        let json = self.weapi_request("song/lyric", data).await?;
        
        let lyric = json.get("lrc").and_then(|l| l.get("lyric"))
            .and_then(|l| l.as_str())
            .unwrap_or("")
            .to_string();
        
        let tlyric = json.get("tlyric").and_then(|t| t.get("lyric"))
            .and_then(|l| l.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        
        // 逐字歌词
        let lxlyric = json.get("klyric").and_then(|k| k.get("lyric"))
            .and_then(|l| l.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        
        Ok(LyricInfo {
            lyric,
            tlyric,
            rlyric: None,
            lxlyric,
        })
    }
    
    async fn get_pic_url(&self, music_info: &MusicInfo) -> Result<String> {
        let id = music_info.id.parse::<u64>().unwrap_or(0);
        Ok(format!(
            "https://p1.music.126.net/{}/{}.jpg",
            crypto::md5(&format!("{}jpg", id)),
            id
        ))
    }
    
    async fn check_available(&self) -> Result<bool> {
        match self.client.get(&self.base_url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl NeteaseSource {
    fn parse_music_info(&self, item: &Value) -> Result<MusicInfo> {
        let id = item.get("id")
            .and_then(|v| v.as_u64())
            .map(|i| i.to_string())
            .ok_or_else(|| Error::ParseFailed("Missing id".to_string()))?;
        
        let name = item.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let singer = item.get("ar")
            .or_else(|| item.get("artists"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        
        let album_name = item.get("al")
            .or_else(|| item.get("album"))
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let interval = item.get("dt")
            .or_else(|| item.get("duration"))
            .and_then(|v| v.as_u64())
            .map(|d| (d / 1000) as u32)
            .unwrap_or(0);
        
        let mut quality = std::collections::HashMap::new();
        quality.insert(MusicQuality::Lq, id.clone());
        
        // 检查音质
        if let Some(h) = item.get("h") {
            if h.get("br").and_then(|b| b.as_u64()).unwrap_or(0) >= 320000 {
                quality.insert(MusicQuality::Hq, id.clone());
            }
        }
        if item.get("sq").is_some() {
            quality.insert(MusicQuality::Sq, id.clone());
        }
        
        Ok(MusicInfo {
            id,
            name,
            singer,
            album_name,
            interval,
            source: MusicSource::Wy,
            quality,
            pic_url: None,
        })
    }
}
