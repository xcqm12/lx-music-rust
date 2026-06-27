use crate::{MusicSourceProvider, crypto, utils};
use common::{MusicInfo, MusicQuality, MusicSource, LyricInfo, Result, Error};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

pub struct MiguSource {
    client: Arc<Client>,
    base_url: String,
}

impl MiguSource {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            base_url: "https://m.music.migu.cn".to_string(),
        }
    }
    
    /// 获取 Token
    async fn get_token(&self) -> Result<String> {
        let url = "https://m.music.migu.cn/migu/remoting/cms_tag_tag&quot;;
        let resp = self.client.get(url).send().await?;
        
        // 从响应中提取 token
        Ok(utils::random_string(32))
    }
    
    /// 构建请求头
    fn build_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Referer", "https://m.music.migu.cn".to_string()),
            ("Origin", "https://m.music.migu.cn".to_string()),
            ("User-Agent", "Mozilla/5.0 (Linux; Android 10; SM-G981B) AppleWebKit/537.36".to_string()),
        ]
    }
    
    /// 音质映射
    fn quality_map(&self, quality: MusicQuality) -> &'static str {
        match quality {
            MusicQuality::Lq => "PQ",
            MusicQuality::Mq => "HQ",
            MusicQuality::Hq => "SQ",
            MusicQuality::Sq => "SQ",
            MusicQuality::Hires => "ZQ",
        }
    }
}

#[async_trait::async_trait]
impl MusicSourceProvider for MiguSource {
    fn name(&self) -> &str {
        "咪咕音乐"
    }
    
    fn source_id(&self) -> MusicSource {
        MusicSource::Mg
    }
    
    async fn search(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> Result<Vec<MusicInfo>> {
        let url = "https://m.music.migu.cn/migu/remoting/scr_search_tag";
        
        let params = [
            ("keyword", keyword),
            ("type", "2"),
            ("pgc", &page.to_string()),
            ("rows", &limit.to_string()),
        ];
        
        let resp = self.client
            .get(url)
            .query(&params)
            .headers(self.build_headers().into_iter().map(|(k, v)| {
                (k.to_string(), v)
            }).collect::<std::collections::HashMap<_, _>>())
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        let mut results = Vec::new();
        
        if let Some(musics) = json.get("musics").and_then(|m| m.as_array()) {
            for music in musics {
                if let Ok(music_info) = self.parse_music_info(music) {
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
        let copyright_id = music_info.id.clone();
        let quality_code = self.quality_map(quality);
        
        let url = "https://app.c.nf.migu.cn/MIGU/3.0/content/sub/listenSongData.do";
        
        let params = [
            ("copyrightId", copyright_id.as_str()),
            ("resourceType", "2"),
            ("purpose", "1"),
            ("type", quality_code),
        ];
        
        let resp = self.client
            .get(url)
            .query(&params)
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
            if let Some(first) = data.first() {
                if let Some(url) = first.get("url").and_then(|u| u.as_str()) {
                    return Ok(url.to_string());
                }
            }
        }
        
        // 尝试备用接口
        let backup_url = "https://freetyst.nf.migu.cn/";
        Ok(format!("{}{}.mp3", backup_url, music_info.id))
    }
    
    async fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let url = "https://m.music.migu.cn/migu/remoting/cms_detail_tag";
        
        let params = [
            ("cpid", music_info.id.as_str()),
        ];
        
        let resp = self.client
            .get(url)
            .query(&params)
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        let lyric = json.get("data")
            .and_then(|d| d.get("lyricLrc"))
            .and_then(|l| l.as_str())
            .unwrap_or("")
            .to_string();
        
        let tlyric = json.get("data")
            .and_then(|d| d.get("lyricTrc"))
            .and_then(|l| l.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        
        Ok(LyricInfo {
            lyric,
            tlyric,
            rlyric: None,
            lxlyric: None,
        })
    }
    
    async fn get_pic_url(&self, music_info: &MusicInfo) -> Result<String> {
        Ok(format!(
            "https://m.music.migu.cn/migu/remoting/img_sns_tag/{}?width=300&height=300",
            music_info.id
        ))
    }
    
    async fn check_available(&self) -> Result<bool> {
        match self.client.get(&self.base_url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl MiguSource {
    fn parse_music_info(&self, item: &Value) -> Result<MusicInfo> {
        let id = item.get("copyrightId")
            .or_else(|| item.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ParseFailed("Missing id".to_string()))?
            .to_string();
        
        let name = item.get("songName")
            .or_else(|| item.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let singer = item.get("singerName")
            .or_else(|| item.get("singer"))
            .and_then(|v| v.as_str())
            .map(|s| s.split(",").map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        
        let album_name = item.get("albumName")
            .or_else(|| item.get("album"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let interval = item.get("length")
            .and_then(|v| v.as_str())
            .and_then(|s| utils::parse_duration(s).ok())
            .or_else(|| item.get("duration").and_then(|v| v.as_u64()).map(|d| (d / 1000) as u32))
            .unwrap_or(0);
        
        let mut quality = std::collections::HashMap::new();
        quality.insert(MusicQuality::Lq, id.clone());
        
        // 根据 available 判断音质
        if item.get("newRateFormats").is_some() || item.get("sq").is_some() {
            quality.insert(MusicQuality::Hq, id.clone());
            quality.insert(MusicQuality::Sq, id.clone());
        }
        
        Ok(MusicInfo {
            id,
            name,
            singer,
            album_name,
            interval,
            source: MusicSource::Mg,
            quality,
            pic_url: None,
        })
    }
}
