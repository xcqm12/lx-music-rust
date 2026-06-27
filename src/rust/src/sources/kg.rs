//! 酷狗音乐 (Kugou) source implementation
//!
//! API endpoints:
//! - Search: https://songsearch.kugou.com/song_search_v2
//! - Lyric: http://m.kugou.com/app/i/krc.php
//! - Pic: http://media.store.kugou.com/v1/get_res_privilege
//! - Music URL: http://trackercdn.kugou.com/i/v2/

use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct KgSearchResponse {
    error_code: Option<i32>,
    data: Option<KgSearchData>,
}

#[derive(Deserialize)]
struct KgSearchData {
    lists: Option<Vec<KgSearchItem>>,
    total: Option<usize>,
}

#[derive(Deserialize)]
struct KgSearchItem {
    #[serde(rename = "Audioid")]
    audioid: Option<String>,
    #[serde(rename = "SongName")]
    song_name: Option<String>,
    #[serde(rename = "Singers")]
    singers: Option<Vec<KgSinger>>,
    #[serde(rename = "AlbumName")]
    album_name: Option<String>,
    #[serde(rename = "AlbumID")]
    album_id: Option<String>,
    #[serde(rename = "Duration")]
    duration: Option<u64>,
    #[serde(rename = "FileHash")]
    file_hash: Option<String>,
    #[serde(rename = "FileSize")]
    file_size: Option<u64>,
    #[serde(rename = "HQFileHash")]
    hq_file_hash: Option<String>,
    #[serde(rename = "HQFileSize")]
    hq_file_size: Option<u64>,
    #[serde(rename = "SQFileHash")]
    sq_file_hash: Option<String>,
    #[serde(rename = "SQFileSize")]
    sq_file_size: Option<u64>,
    #[serde(rename = "ResFileHash")]
    res_file_hash: Option<String>,
    #[serde(rename = "ResFileSize")]
    res_file_size: Option<u64>,
    #[serde(default)]
    #[serde(rename = "Grp")]
    grp: Vec<KgSearchItem>,
}

#[derive(Deserialize)]
struct KgSinger {
    name: Option<String>,
}

#[derive(Deserialize)]
struct KgMusicUrlResponse {
    status: Option<i32>,
    data: Option<KgMusicUrlData>,
}

#[derive(Deserialize)]
struct KgMusicUrlData {
    url: Option<Vec<String>>,
    #[allow(dead_code)]
    #[serde(rename = "extName")]
    ext_name: Option<String>,
}

pub struct KgSource;

impl KgSource {
    pub fn new() -> Self {
        Self
    }

    fn build_quality_info(
        types: &mut Vec<QualityInfo>,
        _types: &mut HashMap<String, QualityInfo>,
        type_name: &str,
        size: u64,
        _hash: Option<String>,
    ) {
        if size == 0 { return; }
        let size_display = size_format(size);
        let qi = QualityInfo {
            quality: type_name.to_string(),
            size: Some(size_display),
            url: None,
        };
        _types.insert(type_name.to_string(), qi.clone());
        types.push(qi);
    }

    fn parse_qualities(&self, item: &KgSearchItem) -> (Vec<QualityInfo>, HashMap<String, QualityInfo>) {
        let mut types = Vec::new();
        let mut _types = HashMap::new();

        Self::build_quality_info(&mut types, &mut _types, "128k", item.file_size.unwrap_or(0), item.file_hash.clone());
        Self::build_quality_info(&mut types, &mut _types, "320k", item.hq_file_size.unwrap_or(0), item.hq_file_hash.clone());
        Self::build_quality_info(&mut types, &mut _types, "flac", item.sq_file_size.unwrap_or(0), item.sq_file_hash.clone());
        Self::build_quality_info(&mut types, &mut _types, "flac24bit", item.res_file_size.unwrap_or(0), item.res_file_hash.clone());

        (types, _types)
    }

    fn format_singers(&self, singers: &[KgSinger]) -> String {
        let names: Vec<&str> = singers.iter()
            .filter_map(|s| s.name.as_deref())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.join("、")
    }
}

impl MusicSourceApi for KgSource {
    fn source_id(&self) -> &str { "kg" }
    fn source_name(&self) -> &str { "酷狗音乐" }

    fn search(&self, keyword: &str, page: usize, limit: usize) -> Result<SearchResult> {
        let url = format!(
            "https://songsearch.kugou.com/song_search_v2?keyword={}&page={}&pagesize={}&userid=0&clientver=&platform=WebFilter&filter=2&iscorrection=1&privilege_filter=0&area_code=1",
            urlencoding(keyword),
            page,
            limit
        );

        let resp: KgSearchResponse = http_get_json(&url)?;

        if resp.error_code.unwrap_or(-1) != 0 {
            return Err(SourceError::SearchFailed("KG search returned error".to_string()));
        }

        let data = resp.data.ok_or_else(|| SourceError::SearchFailed("No data".to_string()))?;
        let lists = data.lists.unwrap_or_default();
        let total = data.total.unwrap_or(0);

        let mut seen_ids = std::collections::HashSet::new();
        let mut result = Vec::new();

        for item in &lists {
            let key = format!("{}_{}", item.audioid.as_deref().unwrap_or(""), item.file_hash.as_deref().unwrap_or(""));
            if seen_ids.contains(&key) { continue; }
            seen_ids.insert(key.clone());

            let (types, _types) = self.parse_qualities(item);

            result.push(MusicInfo {
                id: item.audioid.clone().unwrap_or_default(),
                name: decode_name(item.song_name.as_deref().unwrap_or("")),
                singer: self.format_singers(item.singers.as_deref().unwrap_or(&[])),
                source: "kg".to_string(),
                album_id: item.album_id.clone(),
                album_name: item.album_name.as_ref().map(|a| decode_name(a)),
                duration: item.duration.map(|d| format_play_time(d as f64)),
                pic_url: None,
                lrc_url: None,
                qualitys: types,
                url: None,
            });

            // Process child items
            for child in &item.grp {
                let key = format!("{}_{}", child.audioid.as_deref().unwrap_or(""), child.file_hash.as_deref().unwrap_or(""));
                if seen_ids.contains(&key) { continue; }
                seen_ids.insert(key.clone());

                let (types, _types) = self.parse_qualities(child);
                result.push(MusicInfo {
                    id: child.audioid.clone().unwrap_or_default(),
                    name: decode_name(child.song_name.as_deref().unwrap_or("")),
                    singer: self.format_singers(child.singers.as_deref().unwrap_or(&[])),
                    source: "kg".to_string(),
                    album_id: child.album_id.clone(),
                    album_name: child.album_name.as_ref().map(|a| decode_name(a)),
                    duration: child.duration.map(|d| format_play_time(d as f64)),
                    pic_url: None,
                    lrc_url: None,
                    qualitys: types,
                    url: None,
                });
            }
        }

        Ok(SearchResult {
            source: "kg".to_string(),
            keyword: keyword.to_string(),
            data: result,
            total_count: total,
            page_size: limit,
            page_index: page,
        })
    }

    fn get_music_url(&self, music_info: &MusicInfo, _quality: &str) -> Result<String> {
        // Use the hash-based URL resolution for KG
        let hash = &music_info.id;
        let url = format!(
            "http://trackercdn.kugou.com/i/v2/?hash={}&key=&appid=1001&pid=2&cmd=25&behavior=play",
            hash
        );
        let resp: KgMusicUrlResponse = http_get_json(&url)?;

        match resp.status {
            Some(1) => {
                resp.data
                    .and_then(|d| d.url)
                    .and_then(|urls| urls.into_iter().next())
                    .ok_or_else(|| SourceError::UrlResolutionFailed("No URL returned".to_string()))
            }
            _ => Err(SourceError::UrlResolutionFailed("KG URL resolution failed".to_string())),
        }
    }

    fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        // KG lyric is in KRC format, try to fetch
        let url = format!(
            "http://m.kugou.com/app/i/krc.php?cmd=100&keyword={}&hash={}&timelength=300&d=0.1",
            urlencoding(&music_info.name),
            &music_info.id
        );
        let body = http_get_body(&url)?;

        // Parse KRC format lyric
        let parsed = parse_kg_lyric(&body);
        Ok(LyricInfo {
            lyric: parsed.0,
            translation: parsed.1,
            romaji: None,
            raw_translation: None,
        })
    }

    fn get_pic(&self, music_info: &MusicInfo) -> Result<String> {
        // Try to get pic from KG API
        let url = format!(
            "http://media.store.kugou.com/v1/get_res_privilege"
        );
        let body = serde_json::json!({
            "appid": 1001,
            "area_code": "1",
            "behavior": "play",
            "clientver": "9020",
            "need_hash_offset": 1,
            "relate": 1,
            "resource": [{
                "album_audio_id": music_info.id,
                "album_id": music_info.album_id.as_deref().unwrap_or(""),
                "hash": &music_info.id,
                "id": 0,
                "name": format!("{} - {}.mp3", music_info.singer, music_info.name),
                "type": "audio",
            }],
            "token": "",
            "userid": 2626431536u64,
            "vip": 1,
        });

        let resp = http_utils::post_json(&url, &body.to_string())
            .map_err(|e| SourceError::NetworkError(e))?;

        #[derive(Deserialize)]
        struct KgPicResponse {
            data: Option<Vec<KgPicData>>,
        }
        #[derive(Deserialize)]
        struct KgPicData {
            img: Option<String>,
        }

        let pic_resp: KgPicResponse = serde_json::from_str(&resp.body)
            .map_err(|e| SourceError::ParseError(e.to_string()))?;

        pic_resp.data
            .and_then(|d| d.into_iter().next())
            .and_then(|d| d.img)
            .ok_or_else(|| SourceError::ParseError("No pic data".to_string()))
    }
}

/// Parse Kugou KRC format lyric
fn parse_kg_lyric(body: &str) -> (Option<String>, Option<String>) {
    // KG returns binary KRC format, try simple parsing
    if body.is_empty() || body.starts_with("Error") {
        return (Some("[00:00.00]暂无歌词".to_string()), None);
    }

    // Try to extract plain text from KRC
    let mut lyric = String::new();
    let translation = String::new();

    // Simple KRC parsing: extract text after timestamps
    let re = regex::Regex::new(r"\[\d+,\d+\]([^\[]*)").unwrap();
    for cap in re.captures_iter(body) {
        if let Some(text) = cap.get(1) {
            let t = text.as_str().trim();
            if !t.is_empty() {
                lyric.push_str(t);
                lyric.push('\n');
            }
        }
    }

    if lyric.is_empty() {
        (Some("[00:00.00]暂无歌词".to_string()), None)
    } else {
        (Some(lyric), if translation.is_empty() { None } else { Some(translation) })
    }
}