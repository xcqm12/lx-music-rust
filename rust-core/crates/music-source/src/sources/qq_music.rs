use crate::{MusicSourceProvider, crypto, utils};
use common::{MusicInfo, MusicQuality, MusicSource, LyricInfo, Result, Error};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

pub struct QQMusicSource {
    client: Arc<Client>,
    base_url: String,
}

impl QQMusicSource {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            base_url: "https://u.y.qq.com".to_string(),
        }
    }
    
    /// 获取签名
    fn get_sign(&self, data: &str) -> String {
        // QQ 音乐签名算法
        let key = "ZZBWCYGU";
        let sign_str = format!("{}{}", data, key);
        crypto::md5(&sign_str)
    }
    
    /// GUID
    fn get_guid(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(1000000000..9999999999).to_string()
    }
}

#[async_trait::async_trait]
impl MusicSourceProvider for QQMusicSource {
    fn name(&self) -> &str {
        "QQ音乐"
    }
    
    fn source_id(&self) -> MusicSource {
        MusicSource::Tx
    }
    
    async fn search(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> Result<Vec<MusicInfo>> {
        let url = "https://u.y.qq.com/cgi-bin/musicu.fcg";
        
        let req_data = serde_json::json!({
            "req_1": {
                "method": "DoSearchForQQMusicDesktop",
                "module": "music.search.SearchCgiService",
                "param": {
                    "num_per_page": limit,
                    "page_num": page,
                    "query": keyword,
                    "search_type": 0,
                }
            }
        });
        
        let resp = self.client
            .get(url)
            .query(&[("data", req_data.to_string())])
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        let mut results = Vec::new();
        
        if let Some(list) = json.get("req_1")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.get("body"))
            .and_then(|b| b.get("song"))
            .and_then(|s| s.get("list"))
            .and_then(|l| l.as_array()) {
            
            for item in list {
                if let Ok(music_info) = self.parse_music_info(item) {
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
        let guid = self.get_guid();
        let uin = "0";
        
        let filename = format!("C400{}.m4a", music_info.id);
        let quality_type = match quality {
            MusicQuality::Lq => "128",
            MusicQuality::Mq => "192",
            MusicQuality::Hq => "320",
            MusicQuality::Sq => "flac",
            MusicQuality::Hires => "hires",
        };
        
        let req_data = serde_json::json!({
            "req_0": {
                "module": "vkey.GetVkeyServer",
                "method": "CgiGetVkey",
                "param": {
                    "guid": guid,
                    "songmid": [music_info.id.clone()],
                    "songtype": [0],
                    "uin": uin,
                    "loginflag": 1,
                    "platform": "20",
                }
            }
        });
        
        let url = "https://u.y.qq.com/cgi-bin/musicu.fcg";
        
        let resp = self.client
            .get(url)
            .query(&[("data", req_data.to_string())])
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        if let Some(midurlinfo) = json.get("req_0")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.get("midurlinfo"))
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.first()) {
            
            if let Some(purl) = midurlinfo.get("purl").and_then(|p| p.as_str()) {
                if !purl.is_empty() {
                    return Ok(format!("http://isure.stream.qqmusic.qq.com/{}", purl));
                }
            }
        }
        
        Err(Error::MusicSource("Failed to get music URL".to_string()))
    }
    
    async fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let url = "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg";
        
        let params = [
            ("songmid", music_info.id.as_str()),
            ("pcachetime", &utils::timestamp_ms().to_string()),
            ("g_tk", "5381"),
            ("loginUin", "0"),
            ("hostUin", "0"),
            ("format", "json"),
            ("inCharset", "utf8"),
            ("outCharset", "utf-8"),
        ];
        
        let resp = self.client
            .get(url)
            .query(&params)
            .header("Referer", "https://y.qq.com/portal/player.html")
            .send()
            .await?;
        
        let text = resp.text().await?;
        let json = utils::parse_jsonp(&text).ok_or_else(|| Error::ParseFailed("Invalid JSONP".to_string()))?;
        
        let lyric = json.get("lyric")
            .and_then(|l| l.as_str())
            .and_then(|s| crypto::base64_decode(s).ok())
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default();
        
        let tlyric = json.get("trans")
            .and_then(|t| t.as_str())
            .and_then(|s| crypto::base64_decode(s).ok())
            .and_then(|b| String::from_utf8(b).ok());
        
        Ok(LyricInfo {
            lyric,
            tlyric,
            rlyric: None,
            lxlyric: None,
        })
    }
    
    async fn get_pic_url(&self, music_info: &MusicInfo) -> Result<String> {
        // QQ 音乐封面 URL
        Ok(format!(
            "https://y.gtimg.cn/music/photo_new/T002R300x300M000.jpg",
        ))
    }
    
    async fn check_available(&self) -> Result<bool> {
        match self.client.get("https://y.qq.com").send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl QQMusicSource {
    fn parse_music_info(&self, item: &Value) -> Result<MusicInfo> {
        let mid = item.get("mid")
            .or_else(|| item.get("songmid"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ParseFailed("Missing mid".to_string()))?;
        
        let name = item.get("name")
            .or_else(|| item.get("songname"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let singer = item.get("singer")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        
        let album_name = item.get("album").or_else(|| item.get("albumname"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let interval = item.get("interval")
            .and_then(|v| v.as_i64())
            .map(|i| i as u32)
            .unwrap_or(0);
        
        let mut quality = std::collections::HashMap::new();
        quality.insert(MusicQuality::Lq, mid.to_string());
        
        // 检查可用音质
        if item.get("file").and_then(|f| f.get("size_320mp3")).is_some() {
            quality.insert(MusicQuality::Hq, mid.to_string());
        }
        if item.get("file").and_then(|f| f.get("size_flac")).is_some() {
            quality.insert(MusicQuality::Sq, mid.to_string());
        }
        
        Ok(MusicInfo {
            id: mid.to_string(),
            name,
            singer,
            album_name,
            interval,
            source: MusicSource::Tx,
            quality,
            pic_url: None,
        })
    }
}
