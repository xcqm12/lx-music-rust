use crate::{MusicSourceProvider, crypto, utils};
use common::{MusicInfo, MusicQuality, MusicSource, LyricInfo, Result, Error};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

pub struct KugouSource {
    client: Arc<Client>,
    base_url: String,
}

impl KugouSource {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            base_url: "https://www.kugou.com".to_string(),
        }
    }
    
    /// 计算签名
    fn sign(&self, params: &str) -> String {
        let key = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt";
        let sign_str = format!("{}{}{}", key, params, key);
        crypto::md5(&sign_str)
    }
    
    /// 解析 KRC 歌词
    fn parse_krc(&self, krc_data: &[u8]) -> Result<String> {
        // KRC 是加密的，需要解密
        let key: [u8; 16] = [
            0x40, 0x47, 0x61, 0x77, 0x5e, 0x32, 0x74, 0x47,
            0x51, 0x36, 0x31, 0x2d, 0xce, 0xd2, 0x6e, 0x69,
        ];
        
        // 跳过前4字节头部
        if krc_data.len() < 4 {
            return Ok(String::new());
        }
        
        let encrypted = &krc_data[4..];
        let decrypted = crypto::aes_ecb_decrypt(encrypted, &key);
        
        // 解压 zlib
        let mut decoder = flate2::read::ZlibDecoder::new(&decrypted[..]);
        let mut result = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut result)
            .map_err(|e| Error::ParseFailed(e.to_string()))?;
        
        Ok(result)
    }
}

#[async_trait::async_trait]
impl MusicSourceProvider for KugouSource {
    fn name(&self) -> &str {
        "酷狗音乐"
    }
    
    fn source_id(&self) -> MusicSource {
        MusicSource::Kg
    }
    
    async fn search(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> Result<Vec<MusicInfo>> {
        let url = "https://complexsearch.kugou.com/v2/search/song";
        
        let timestamp = utils::timestamp_ms().to_string();
        let params = format!(
            "appid=1014&bitrate=0&callback=&clienttime={}&clientver=1000&dfid=&filter=10&inputtype=0&iscorrection=1&isfuzzy=0&keyword={}&mid={}&page={}&pagesize={}&platform=WebFilter&privilege_filter=0&srcappid=2919&token=&userid=0",
            timestamp,
            urlencoding::encode(keyword),
            utils::random_string(16),
            page,
            limit
        );
        
        let signature = self.sign(&params);
        
        let resp = self.client
            .get(url)
            .query(&[("signature", &signature)])
            .query(&serde_urlencoded::from_str::<Vec<(String, String)>>(&params).unwrap_or_default())
            .send()
            .await?;
        
        let text = resp.text().await?;
        let json_str = utils::parse_jsonp(&text).ok_or_else(|| Error::ParseFailed("Invalid JSONP".to_string()))?;
        
        let mut results = Vec::new();
        
        if let Some(list) = json_str.get("data").and_then(|d| d.get("lists")).and_then(|l| l.as_array()) {
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
        let hash = music_info.quality.get(&quality)
            .or_else(|| music_info.quality.get(&MusicQuality::Hq))
            .or_else(|| music_info.quality.get(&MusicQuality::Mq))
            .or_else(|| music_info.quality.get(&MusicQuality::Lq))
            .ok_or_else(|| Error::MusicSource("No quality available".to_string()))?;
        
        let url = "https://wwwapi.kugou.com/yy/index.php";
        
        let mid = utils::random_string(16);
        let params = [
            ("r", "play/getdata"),
            ("hash", hash.as_str()),
            ("dfid", ""),
            ("appid", "1014"),
            ("mid", mid.as_str()),
        ];
        
        let resp = self.client
            .get(url)
            .query(&params)
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        json.get("data")
            .and_then(|d| d.get("play_url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::MusicSource("Failed to get music URL".to_string()))
    }
    
    async fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let url = "https://krcs.kugou.com/search";
        
        let params = [
            ("ver", "1"),
            ("man", "yes"),
            ("client", "mobi"),
            ("keyword", &music_info.name),
            ("duration", &(music_info.interval * 1000).to_string()),
            ("hash", music_info.id.as_str()),
        ];
        
        let resp = self.client
            .get(url)
            .query(&params)
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        // 解析歌词
        let mut lyric = String::new();
        let mut tlyric = None;
        
        if let Some(candidates) = json.get("candidates").and_then(|c| c.as_array()) {
            if let Some(first) = candidates.first() {
                if let Some(access_key) = first.get("accesskey").and_then(|k| k.as_str()) {
                    if let Some(id) = first.get("id").and_then(|i| i.as_str()) {
                        // 下载 KRC 歌词
                        let krc_url = format!(
                            "https://lyrics.kugou.com/download?ver=1&client=pc&id={}&accesskey={}&fmt=krc&charset=utf8",
                            id, access_key
                        );
                        
                        if let Ok(krc_resp) = self.client.get(&krc_url).send().await {
                            if let Ok(krc_json) = krc_resp.json::<Value>().await {
                                if let Some(content) = krc_json.get("content").and_then(|c| c.as_str()) {
                                    if let Ok(krc_bytes) = crypto::base64_decode(content) {
                                        if let Ok(parsed) = self.parse_krc(&krc_bytes) {
                                            lyric = parsed;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(LyricInfo {
            lyric,
            tlyric,
            rlyric: None,
            lxlyric: None,
        })
    }
    
    async fn get_pic_url(&self, music_info: &MusicInfo) -> Result<String> {
        Ok(format!(
            "https://albumcover.kugou.com/oss/albumcover/{}.jpg",
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

impl KugouSource {
    fn parse_music_info(&self, item: &Value) -> Result<MusicInfo> {
        let hash = item.get("FileHash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let name = item.get("SongName")
            .and_then(|v| v.as_str())
            .map(|s| utils::strip_html(s))
            .unwrap_or_default();
        
        let singer = item.get("SingerName")
            .and_then(|v| v.as_str())
            .map(|s| s.split("、").map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        
        let album_name = item.get("AlbumName")
            .and_then(|v| v.as_str())
            .map(|s| utils::strip_html(s))
            .unwrap_or_default();
        
        let interval = item.get("Duration")
            .and_then(|v| v.as_i64())
            .map(|i| i as u32)
            .unwrap_or(0);
        
        let mut quality = std::collections::BTreeMap::new();
        
        // 不同音质对应的 hash
        if let Some(hq_hash) = item.get("HQFileHash").and_then(|v| v.as_str()) {
            if !hq_hash.is_empty() {
                quality.insert(MusicQuality::Hq, hq_hash.to_string());
            }
        }
        if let Some(sq_hash) = item.get("SQFileHash").and_then(|v| v.as_str()) {
            if !sq_hash.is_empty() {
                quality.insert(MusicQuality::Sq, sq_hash.to_string());
            }
        }
        
        // 默认使用 FileHash
        quality.insert(MusicQuality::Lq, hash.clone());
        
        Ok(MusicInfo {
            id: hash,
            name,
            singer,
            album_name,
            interval,
            source: MusicSource::Kg,
            quality,
            pic_url: None,
        })
    }
}
