//! 咪咕音乐 (Migu) source implementation
//!
//! API endpoints:
//! - Search: https://app.c.nf.migu.cn/MIGUM2.0/v1.0/content/search_all.do
//! - Lyric: https://music.migu.cn/v3/api/music/audioPlayer/getLyric
//! - Pic: https://music.migu.cn/v3/api/music/audioPlayer/getSongPic

use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct MgSearchResponse {
    #[allow(dead_code)]
    code: Option<String>,
    #[serde(rename = "songResultData")]
    song_result_data: Option<MgSongResultData>,
}

#[derive(Deserialize)]
struct MgSongResultData {
    result: Option<Vec<MgSearchItem>>,
    #[serde(rename = "totalCount")]
    total_count: Option<usize>,
}

#[derive(Deserialize)]
struct MgSearchItem {
    #[allow(dead_code)]
    #[serde(rename = "id")]
    id: Option<String>,
    #[serde(rename = "songId")]
    song_id: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "copyrightId")]
    copyright_id: Option<String>,
    name: Option<String>,
    singers: Option<Vec<MgSinger>>,
    albums: Option<Vec<MgAlbum>>,
    #[serde(rename = "newRateFormats")]
    new_rate_formats: Option<Vec<MgRateFormat>>,
    #[serde(rename = "imgItems")]
    img_items: Option<Vec<MgImgItem>>,
    #[serde(rename = "lyricUrl")]
    lyric_url: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "duration")]
    duration: Option<String>,
}

#[derive(Deserialize)]
struct MgSinger {
    name: Option<String>,
}

#[derive(Deserialize)]
struct MgAlbum {
    id: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct MgRateFormat {
    #[serde(rename = "formatType")]
    format_type: Option<String>,
    size: Option<u64>,
    #[serde(rename = "androidSize")]
    android_size: Option<u64>,
}

#[derive(Deserialize)]
struct MgImgItem {
    img: Option<String>,
}

#[derive(Deserialize)]
struct MgLyricResponse {
    code: Option<String>,
    #[serde(rename = "lyricData")]
    lyric_data: Option<String>,
}

pub struct MgSource;

impl MgSource {
    pub fn new() -> Self {
        Self
    }

    fn format_singers(&self, singers: &[MgSinger]) -> String {
        let names: Vec<&str> = singers.iter()
            .filter_map(|s| s.name.as_deref())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.join("、")
    }

    fn parse_quality_infos(&self, formats: &[MgRateFormat]) -> (Vec<QualityInfo>, HashMap<String, QualityInfo>) {
        let mut types = Vec::new();
        let mut _types = HashMap::new();

        for fmt in formats {
            let type_name = match fmt.format_type.as_deref() {
                Some("PQ") => "128k",
                Some("HQ") => "320k",
                Some("SQ") => "flac",
                Some("ZQ") => "flac24bit",
                _ => continue,
            };
            let size = fmt.size.or(fmt.android_size).unwrap_or(0);
            let size_display = if size > 0 { Some(size_format(size)) } else { None };
            let qi = QualityInfo {
                quality: type_name.to_string(),
                size: size_display,
                url: None,
            };
            _types.insert(type_name.to_string(), qi.clone());
            types.push(qi);
        }

        (types, _types)
    }
}

impl MusicSourceApi for MgSource {
    fn source_id(&self) -> &str { "mg" }
    fn source_name(&self) -> &str { "咪咕音乐" }

    fn search(&self, keyword: &str, page: usize, limit: usize) -> Result<SearchResult> {
        let url = format!(
            "https://app.c.nf.migu.cn/MIGUM2.0/v1.0/content/search_all.do?isCopyright=1&isCorrect=1&pageNo={}&pageSize={}&searchSwitch={{\"song\":1,\"album\":0,\"singer\":0,\"tagSong\":0,\"mvSong\":0,\"songlist\":0,\"bestShow\":1}}&sort=0&text={}",
            page,
            limit,
            urlencoding(keyword)
        );

        let resp: MgSearchResponse = http_get_json(&url)?;

        let song_data = resp.song_result_data.ok_or_else(||
            SourceError::SearchFailed("No song result data".to_string()))?;

        let items = song_data.result.unwrap_or_default();
        let total = song_data.total_count.unwrap_or(0);

        let mut seen_ids = std::collections::HashSet::new();
        let mut data = Vec::new();

        for item in items {
            let song_id = item.song_id.as_deref().unwrap_or("");
            if song_id.is_empty() || seen_ids.contains(song_id) { continue; }
            seen_ids.insert(song_id.to_string());

            let (types, _types) = item.new_rate_formats.as_ref()
                .map(|f| self.parse_quality_infos(f))
                .unwrap_or_default();

            let album_info = item.albums.as_ref().and_then(|a| a.first());
            let img = item.img_items.as_ref()
                .and_then(|imgs| imgs.first())
                .and_then(|i| i.img.clone());

            data.push(MusicInfo {
                id: song_id.to_string(),
                name: item.name.unwrap_or_default(),
                singer: self.format_singers(item.singers.as_deref().unwrap_or(&[])),
                source: "mg".to_string(),
                album_id: album_info.and_then(|a| a.id.clone()),
                album_name: album_info.and_then(|a| a.name.clone()),
                duration: None,
                pic_url: img,
                lrc_url: item.lyric_url.clone(),
                qualitys: types,
                url: None,
            });
        }

        Ok(SearchResult {
            source: "mg".to_string(),
            keyword: keyword.to_string(),
            data,
            total_count: total,
            page_size: limit,
            page_index: page,
        })
    }

    fn get_music_url(&self, music_info: &MusicInfo, _quality: &str) -> Result<String> {
        let url = format!(
            "https://music.migu.cn/v3/api/music/audioPlayer/getPlayInfo?copyrightId={}&resourceType=2",
            &music_info.id
        );
        let body = http_get_body(&url)?;

        #[derive(Deserialize)]
        struct MgPlayInfoResponse {
            data: Option<MgPlayInfoData>,
        }
        #[derive(Deserialize)]
        struct MgPlayInfoData {
            #[serde(rename = "playUrl")]
            play_url: Option<String>,
        }

        let resp: MgPlayInfoResponse = serde_json::from_str(&body)
            .map_err(|e| SourceError::ParseError(e.to_string()))?;

        resp.data
            .and_then(|d| d.play_url)
            .ok_or_else(|| SourceError::UrlResolutionFailed("No play URL".to_string()))
    }

    fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let url = format!(
            "https://music.migu.cn/v3/api/music/audioPlayer/getLyric?copyrightId={}",
            &music_info.id
        );
        let body = http_get_body(&url)?;

        let resp: MgLyricResponse = serde_json::from_str(&body)
            .map_err(|e| SourceError::ParseError(e.to_string()))?;

        match resp.code.as_deref() {
            Some("000000") => {
                Ok(LyricInfo {
                    lyric: resp.lyric_data,
                    translation: None,
                    romaji: None,
                    raw_translation: None,
                })
            }
            _ => Ok(LyricInfo {
                lyric: Some("[00:00.00]暂无歌词".to_string()),
                translation: None,
                romaji: None,
                raw_translation: None,
            }),
        }
    }

    fn get_pic(&self, music_info: &MusicInfo) -> Result<String> {
        if let Some(pic_url) = &music_info.pic_url {
            if !pic_url.is_empty() {
                return Ok(pic_url.clone());
            }
        }

        let url = format!(
            "https://music.migu.cn/v3/api/music/audioPlayer/getSongPic?songId={}",
            &music_info.id
        );
        let body = http_get_body(&url)?;

        #[derive(Deserialize)]
        struct MgPicResponse {
            data: Option<MgPicData>,
        }
        #[derive(Deserialize)]
        struct MgPicData {
            img: Option<String>,
            #[serde(rename = "smallPic")]
            small_pic: Option<String>,
            #[serde(rename = "largePic")]
            large_pic: Option<String>,
        }

        let resp: MgPicResponse = serde_json::from_str(&body)
            .map_err(|e| SourceError::ParseError(e.to_string()))?;

        resp.data.as_ref()
            .and_then(|d| d.large_pic.as_ref().or(d.img.as_ref()).or(d.small_pic.as_ref()))
            .cloned()
            .ok_or_else(|| SourceError::ParseError("No pic data".to_string()))
    }
}