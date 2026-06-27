//! 酷我音乐 (Kuwo) source implementation
//!
//! API endpoints:
//! - Search: http://search.kuwo.cn/r.s
//! - Lyric: http://m.kuwo.cn/newh5/singles/songinfoandlrc
//! - Pic: http://artistpicserver.kuwo.cn/pic.web
//! - Music URL: http://www.kuwo.cn/api/v1/www/music/playUrl

use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct KwSearchResponse {
    #[serde(rename = "TOTAL")]
    total: String,
    #[allow(dead_code)]
    #[serde(rename = "SHOW")]
    show: String,
    abslist: Option<Vec<KwSearchItem>>,
}

#[derive(Deserialize)]
struct KwSearchItem {
    #[serde(rename = "MUSICRID")]
    musicrid: String,
    #[serde(rename = "SONGNAME")]
    songname: String,
    #[serde(rename = "ARTIST")]
    artist: String,
    #[serde(rename = "ALBUM")]
    album: Option<String>,
    #[serde(rename = "ALBUMID")]
    albumid: Option<String>,
    #[serde(rename = "DURATION")]
    duration: Option<String>,
    #[serde(rename = "N_MINFO")]
    n_minfo: Option<String>,
}

#[derive(Deserialize)]
struct KwLyricResponse {
    #[allow(dead_code)]
    status: Option<i32>,
    data: Option<KwLyricData>,
}

#[derive(Deserialize)]
struct KwLyricData {
    lrclist: Option<Vec<KwLrcLine>>,
    songinfo: Option<KwSongInfo>,
}

#[derive(Deserialize)]
struct KwLrcLine {
    time: f64,
    #[serde(rename = "lineLyric")]
    line_lyric: String,
}

#[derive(Deserialize)]
struct KwSongInfo {
    #[serde(rename = "songName")]
    song_name: Option<String>,
    artist: Option<String>,
    album: Option<String>,
}

#[derive(Deserialize)]
struct KwMusicUrlResponse {
    code: Option<i32>,
    msg: Option<String>,
    data: Option<KwMusicUrlData>,
}

#[derive(Deserialize)]
struct KwMusicUrlData {
    url: Option<String>,
}

pub struct KwSource;

impl KwSource {
    pub fn new() -> Self {
        Self
    }

    fn parse_music_id(&self, musicrid: &str) -> String {
        musicrid.replace("MUSIC_", "")
    }

    fn parse_quality_infos(&self, n_minfo: &str) -> (Vec<QualityInfo>, HashMap<String, QualityInfo>) {
        let mut types = Vec::new();
        let mut _types = HashMap::new();
        let re = regex::Regex::new(r"level:(\w+),bitrate:(\d+),format:(\w+),size:([\w.]+)").unwrap();

        for caps in re.captures_iter(n_minfo) {
            let _level = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let bitrate: u32 = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let size_str = caps.get(4).map(|m| m.as_str()).unwrap_or("");

            let type_name = match bitrate {
                4000 => "flac24bit",
                2000 => "flac",
                320 => "320k",
                128 => "128k",
                _ => continue,
            };

            let size_bytes = parse_size(size_str);
            let size_display = size_bytes.map(|b| size_format(b));
            let qi = QualityInfo {
                quality: type_name.to_string(),
                size: size_display,
                url: None,
            };
            _types.insert(type_name.to_string(), qi.clone());
            types.push(qi);
        }

        types.reverse();
        (types, _types)
    }
}

impl MusicSourceApi for KwSource {
    fn source_id(&self) -> &str { "kw" }
    fn source_name(&self) -> &str { "酷我音乐" }

    fn search(&self, keyword: &str, page: usize, limit: usize) -> Result<SearchResult> {
        let url = format!(
            "http://search.kuwo.cn/r.s?client=kt&all={}&pn={}&rn={}&uid=794762570&ver=kwplayer_ar_9.2.2.1&vipver=1&show_copyright_off=1&newver=1&ft=music&cluster=0&strategy=2012&encoding=utf8&rformat=json&vermerge=1&mobi=1&issubtitle=1",
            urlencoding(keyword),
            page.saturating_sub(1),
            limit
        );

        let resp: KwSearchResponse = http_get_json(&url)?;

        let total: usize = resp.total.parse().unwrap_or(0);

        let mut data = Vec::new();
        if let Some(items) = resp.abslist {
            for item in items {
                let song_id = self.parse_music_id(&item.musicrid);
                let (types, _types) = if let Some(ref minfo) = item.n_minfo {
                    self.parse_quality_infos(minfo)
                } else {
                    (Vec::new(), HashMap::new())
                };
                let _ = _types;

                let duration = item.duration
                    .and_then(|d| d.parse::<f64>().ok())
                    .map(|d| format_play_time(d));

                data.push(MusicInfo {
                    id: song_id.clone(),
                    name: decode_name(&item.songname),
                    singer: format_singer(&decode_name(&item.artist)),
                    source: "kw".to_string(),
                    album_id: item.albumid.map(|a| decode_name(&a)),
                    album_name: item.album.map(|a| decode_name(&a)),
                    duration,
                    pic_url: None,
                    lrc_url: None,
                    qualitys: types,
                    url: None,
                });
            }
        }

        Ok(SearchResult {
            source: "kw".to_string(),
            keyword: keyword.to_string(),
            data,
            total_count: total,
            page_size: limit,
            page_index: page,
        })
    }

    fn get_music_url(&self, music_info: &MusicInfo, quality: &str) -> Result<String> {
        let br = quality_to_br(quality, "kw");
        let url = format!(
            "http://www.kuwo.cn/api/v1/www/music/playUrl?mid={}&type=music&br={}",
            music_info.id, br
        );
        let resp: KwMusicUrlResponse = http_get_json(&url)?;
        match resp.code {
            Some(200) => {
                resp.data
                    .and_then(|d| d.url)
                    .ok_or_else(|| SourceError::UrlResolutionFailed("No URL returned".to_string()))
            }
            _ => Err(SourceError::UrlResolutionFailed(
                resp.msg.unwrap_or_else(|| "Unknown error".to_string())
            )),
        }
    }

    fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let url = format!(
            "http://m.kuwo.cn/newh5/singles/songinfoandlrc?musicId={}",
            &music_info.id
        );
        let resp: KwLyricResponse = http_get_json(&url)?;

        let data = resp.data.ok_or_else(|| SourceError::ParseError("No lyric data".to_string()))?;
        let lrclist = data.lrclist.unwrap_or_default();

        if lrclist.is_empty() {
            return Ok(LyricInfo {
                lyric: Some("[00:00.00]暂无歌词".to_string()),
                translation: None,
                romaji: None,
                raw_translation: None,
            });
        }

        let (lrc, lrc_t) = sort_lrc_arr(&lrclist);

        let song_name = data.songinfo
            .as_ref()
            .and_then(|s| s.song_name.as_deref())
            .unwrap_or("Unknown");
        let artist = data.songinfo
            .as_ref()
            .and_then(|s| s.artist.as_deref())
            .unwrap_or("Unknown");
        let album = data.songinfo
            .as_ref()
            .and_then(|s| s.album.as_deref())
            .unwrap_or("");

        let lyric = transform_lrc(song_name, artist, album, &lrc);
        let tlyric = if lrc_t.is_empty() {
            String::new()
        } else {
            transform_lrc(song_name, artist, album, &lrc_t)
        };

        Ok(LyricInfo {
            lyric: Some(decode_name(&lyric)),
            translation: if tlyric.is_empty() { None } else { Some(decode_name(&tlyric)) },
            romaji: None,
            raw_translation: None,
        })
    }

    fn get_pic(&self, music_info: &MusicInfo) -> Result<String> {
        let url = format!(
            "http://artistpicserver.kuwo.cn/pic.web?corp=kuwo&type=rid_pic&pictype=500&size=500&rid={}",
            &music_info.id
        );
        let body = http_get_body(&url)?;
        if body.starts_with("http") {
            Ok(body)
        } else {
            Err(SourceError::ParseError("Invalid pic URL returned".to_string()))
        }
    }
}

/// Sort LRC array: separate main lyrics and translations
fn sort_lrc_arr(lines: &[KwLrcLine]) -> (Vec<KwLrcLine>, Vec<KwLrcLine>) {
    let mut lrc = Vec::new();
    let mut lrc_t = Vec::new();
    let mut seen_times = std::collections::HashSet::new();

    for line in lines {
        let time_key = (line.time * 1000.0) as u64;
        if seen_times.contains(&time_key) {
            if let Some(prev) = lrc.pop() {
                lrc_t.push(prev);
            }
            lrc.push(KwLrcLine {
                time: line.time,
                line_lyric: line.line_lyric.clone(),
            });
        } else {
            lrc.push(KwLrcLine {
                time: line.time,
                line_lyric: line.line_lyric.clone(),
            });
            seen_times.insert(time_key);
        }
    }

    (lrc, lrc_t)
}

/// Transform LRC lines to LRC format string
fn transform_lrc(song_name: &str, artist: &str, album: &str, lines: &[KwLrcLine]) -> String {
    let mut result = format!(
        "[ti:{}]\n[ar:{}]\n[al:{}]\n[by:]\n[offset:0]\n",
        song_name, artist, album
    );
    for line in lines {
        let m = (line.time / 60.0) as u64;
        let s = line.time % 60.0;
        result.push_str(&format!("[{:02}:{:05.2?}]{}\n", m, s, line.line_lyric));
    }
    result
}

/// Convert quality string to bitrate for Kuwo API
fn quality_to_br(quality: &str, _source: &str) -> String {
    match quality {
        "flac24bit" => "4000k",
        "flac" => "2000k",
        "320k" => "320k",
        "128k" => "128k",
        _ => "320k",
    }.to_string()
}