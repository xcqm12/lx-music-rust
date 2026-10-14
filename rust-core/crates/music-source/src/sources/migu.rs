use crate::{MusicSourceProvider, crypto};
use common::{MusicInfo, MusicQuality, MusicSource, LyricInfo, Result, Error};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// 上游 mg/musicSearch.js v3 签名参数（与 JS 1:1）
const DEVICE_ID: &str = "963B7AA0D21511ED807EE5846EC87D20";
const SIGN_SECRET: &str = "6cdc72a439cef99a3418d2a78aa28c73yyapp2d16148780a1dcc7408e06336b98cfd50";
const SEARCH_UA: &str = "Mozilla/5.0 (Linux; U; Android 11.0.0; zh-cn; MI 11 Build/OPR1.170623.032) AppleWebKit/534.30 (KHTML, like Gecko) Version/4.0 Mobile Safari/534.30";
const LYRIC_UA: &str = "Mozilla/5.0 (Linux; Android 5.1.1; Nexus 6 Build/LYZ28E) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36";
const PICTURE_HOST: &str = "http://d.musicapp.migu.cn";

static MRC_LINE_TIME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*\[(\d+),\d+\]").unwrap());
static MRC_WORD_TIME: Lazy<Regex> = Lazy::new(|| Regex::new(r"\((\d+),(\d+)\)").unwrap());

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

    /// md5(text + secret + deviceId + time)，同上游 createSignature
    fn create_signature(time: &str, text: &str) -> String {
        crypto::md5(&format!("{}{}{}{}", text, SIGN_SECRET, DEVICE_ID, time))
    }

    /// encodeURIComponent 口径（保留 A-Za-z0-9 -_.!~*'()）
    fn encode_uri_component(s: &str) -> String {
        let mut out = String::with_capacity(s.len() * 3);
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
                | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }

    /// 歌词下载请求头（同上游 mrcTools.getText）
    fn lyric_headers() -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::REFERER, "https://app.c.nf.migu.cn/".parse().unwrap());
        headers.insert(reqwest::header::USER_AGENT, LYRIC_UA.parse().unwrap());
        headers.insert("channel", "0146921".parse().unwrap());
        headers
    }

    /// resourceinfo.do（同上游 mg/musicInfo.js getMusicInfo，resourceId 必须为 songId）
    async fn get_resource_info(&self, song_id: &str) -> Result<Value> {
        let url = "https://c.musicapp.migu.cn/MIGUM2.0/v1.0/content/resourceinfo.do?resourceType=2";
        let resp = self.client
            .post(url)
            .form(&[("resourceId", song_id)])
            .send()
            .await?;
        let json: Value = resp.json().await?;

        json.get("resource")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .or_else(|| {
                json.get("data")
                    .and_then(|d| d.get("resource"))
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
            })
            .cloned()
            .ok_or_else(|| Error::MusicSource("resourceinfo 返回为空".to_string()))
    }

    /// 下载文本，同上游 getText：非 200 最多重试至 tryNum > 5，404 立即失败
    async fn get_text(&self, url: &str) -> Result<String> {
        let mut tries = 0u32;
        loop {
            match self.client.get(url).headers(Self::lyric_headers()).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp.text().await?);
                    }
                    if status.as_u16() == 404 {
                        return Err(Error::Lyric("歌词获取失败".to_string()));
                    }
                }
                Err(_) => {}
            }
            tries += 1;
            if tries > 6 {
                return Err(Error::Lyric("歌词获取失败".to_string()));
            }
        }
    }

    /// 上游 musicSearch.js filterData：resultList 二维数组按 copyrightId 去重
    fn filter_data(result_list: &[Value]) -> Vec<MusicInfo> {
        let mut list = Vec::new();
        let mut ids = HashSet::new();

        for item in result_list {
            let group = match item.as_array() {
                Some(g) => g,
                None => continue,
            };
            for data in group {
                let song_id = match val_to_string(data.get("songId")) {
                    Some(v) if !v.is_empty() => v,
                    _ => continue,
                };
                let copyright_id = match val_to_string(data.get("copyrightId")) {
                    Some(v) if !v.is_empty() => v,
                    _ => continue,
                };
                if !ids.insert(copyright_id.clone()) {
                    continue;
                }

                let mut quality = BTreeMap::new();
                if let Some(formats) = data.get("audioFormats").and_then(|v| v.as_array()) {
                    for f in formats {
                        let q = match f.get("formatType").and_then(|v| v.as_str()) {
                            Some("PQ") => MusicQuality::Lq,
                            Some("HQ") => MusicQuality::Hq,
                            Some("SQ") => MusicQuality::Sq,
                            Some("ZQ24") | Some("ZQ") => MusicQuality::Hires,
                            _ => continue,
                        };
                        quality.insert(q, copyright_id.clone());
                    }
                }

                let singer = data.get("singerList")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|it| {
                                it.get("name")
                                    .and_then(|v| v.as_str())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string())
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                // img3 || img2 || img1，非 http 开头补默认图床
                let mut img = String::new();
                for key in ["img3", "img2", "img1"] {
                    if let Some(v) = data.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                        img = v.to_string();
                        break;
                    }
                }
                let pic_url = if img.is_empty() {
                    None
                } else if img.starts_with("http") {
                    Some(img)
                } else {
                    Some(format!("{}{}", PICTURE_HOST, img))
                };

                let interval = val_to_string(data.get("duration"))
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0);

                list.push(MusicInfo {
                    id: song_id,
                    name: val_to_string(data.get("name")).unwrap_or_default(),
                    singer,
                    album_name: val_to_string(data.get("album")).unwrap_or_default(),
                    interval,
                    source: MusicSource::Mg,
                    quality,
                    pic_url,
                });
            }
        }

        list
    }
}

fn val_to_string(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

// ===== MRC（XXTEA）解密，同上游 mg/utils/mrc.js =====

const MRC_DELTA: i64 = 0x9E3779B9_u32 as i64; // 2654435769
const MRC_MIN_LENGTH: usize = 32;
const MRC_KEY: [i64; 9] = [
    27303562373562475,
    18014862372307051,
    22799692160172081,
    34058940340699235,
    30962724186095721,
    27303523720101991,
    27303523720101998,
    31244139033526382,
    28992395054481524,
];

/// teaDecrypt：JS 侧 BigInt 全程 toLong（mod 2^64 signed wrap），
/// 对应 i64 的 wrapping_add/sub 与算术右移
fn tea_decrypt(data: &mut [i64]) {
    let length = data.len();
    if length < 1 {
        return;
    }
    let mut y = data[0];
    let mut sum = (6 + 52 / length as i64).wrapping_mul(MRC_DELTA);
    while sum != 0 {
        let e = (sum >> 2) & 3;
        for p in (1..length).rev() {
            let z = data[p - 1];
            let left = (y ^ sum).wrapping_add(z ^ MRC_KEY[(((p as i64) & 3) ^ e) as usize]);
            let right = ((z >> 5) ^ y.wrapping_shl(2))
                .wrapping_add((y >> 3) ^ z.wrapping_shl(4));
            let updated = data[p].wrapping_sub(left ^ right);
            data[p] = updated;
            y = updated;
        }
        let z = data[length - 1];
        let left = (y ^ sum).wrapping_add(z ^ MRC_KEY[e as usize]);
        let right = ((z >> 5) ^ y.wrapping_shl(2))
            .wrapping_add((y >> 3) ^ z.wrapping_shl(4));
        let updated = data[0].wrapping_sub(left ^ right);
        data[0] = updated;
        y = updated;
        sum = sum.wrapping_sub(MRC_DELTA);
    }
}

/// 8 字节小端 long → utf16le 字符串（每 word 独立解码，与 JS 一致）
fn long_to_utf16(v: i64) -> String {
    let b = v.to_le_bytes();
    let units = [
        u16::from_le_bytes([b[0], b[1]]),
        u16::from_le_bytes([b[2], b[3]]),
        u16::from_le_bytes([b[4], b[5]]),
        u16::from_le_bytes([b[6], b[7]]),
    ];
    char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// 同上游 decrypt：hex 文本每 16 字符 → signed i64，XXTEA 解密后按 utf16le 拼接
fn decrypt_mrc(data: &str) -> Result<String> {
    if data.len() < MRC_MIN_LENGTH {
        return Ok(data.to_string());
    }
    let count = data.len() / 16;
    let mut words = Vec::with_capacity(count);
    for i in 0..count {
        let chunk = &data[i * 16..(i + 1) * 16];
        let bytes = crypto::hex_decode(chunk)
            .map_err(|_| Error::LyricParseFailed("mrc hex 解码失败".to_string()))?;
        if bytes.len() != 8 {
            return Err(Error::LyricParseFailed("mrc hex 解码失败".to_string()));
        }
        words.push(i64::from_be_bytes(bytes[..8].try_into().unwrap()));
    }
    tea_decrypt(&mut words);
    let mut out = String::new();
    for w in words {
        out.push_str(&long_to_utf16(w));
    }
    Ok(out)
}

// ===== mrcTools.parseLyric，同上游 mg/lyric.js =====

fn parse_mrc_lyric(str: &str) -> (String, String) {
    let str = str.replace('\r', "");
    let mut lrc_lines: Vec<String> = Vec::new();
    let mut lxlrc_lines: Vec<String> = Vec::new();

    for line in str.split('\n') {
        if line.chars().count() < 6 {
            continue;
        }
        let caps = match MRC_LINE_TIME.captures(line) {
            Some(c) => c,
            None => continue,
        };
        let start_time: i64 = match caps[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let time = format!(
            "{:02}:{:02}.{:03}",
            start_time / 60000,
            (start_time / 1000) % 60,
            start_time % 1000
        );

        let words = MRC_LINE_TIME.replace(line, "");
        lrc_lines.push(format!("[{}]{}", time, MRC_WORD_TIME.replace_all(&words, "")));

        if !MRC_WORD_TIME.is_match(&words) {
            continue;
        }
        let parts: Vec<&str> = MRC_WORD_TIME.split(&words).collect();
        let mut lx = String::new();
        for (i, c) in MRC_WORD_TIME.captures_iter(&words).enumerate() {
            let word_start: i64 = c[1].parse().unwrap_or(0);
            lx.push_str(&format!(
                "<{},{}>{}",
                word_start - start_time,
                &c[2],
                parts.get(i).copied().unwrap_or("")
            ));
        }
        lxlrc_lines.push(format!("[{}]{}", time, lx));
    }

    (lrc_lines.join("\n"), lxlrc_lines.join("\n"))
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
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis().to_string())
            .unwrap_or_else(|_| "0".to_string());
        let sign = Self::create_signature(&time, keyword);
        let url = format!(
            "https://jadeite.migu.cn/music_search/v3/search/searchAll?isCorrect=0&isCopyright=1\
             &searchSwitch=%7B%22song%22%3A1%2C%22album%22%3A0%2C%22singer%22%3A0\
             %2C%22tagSong%22%3A1%2C%22mvSong%22%3A0%2C%22bestShow%22%3A1%2C%22songlist%22%3A0\
             %2C%22lyricSong%22%3A0%7D&pageSize={}&text={}&pageNo={}&sort=0&sid=USS",
            limit,
            Self::encode_uri_component(keyword),
            page
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("uiVersion", "A_music_3.6.1".parse().unwrap());
        headers.insert("deviceId", DEVICE_ID.parse().unwrap());
        headers.insert("timestamp", time.parse().unwrap());
        headers.insert("sign", sign.parse().unwrap());
        headers.insert("channel", "0146921".parse().unwrap());
        headers.insert(reqwest::header::USER_AGENT, SEARCH_UA.parse().unwrap());

        let resp = self.client.get(&url).headers(headers).send().await?;
        let json: Value = resp.json().await?;

        if json.get("code").and_then(|v| v.as_str()) != Some("000000") {
            let info = json.get("info")
                .and_then(|v| v.as_str())
                .unwrap_or("搜索失败");
            return Err(Error::MusicSource(info.to_string()));
        }

        let result_list = json.get("songResultData")
            .and_then(|d| d.get("resultList"))
            .and_then(|v| v.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);
        Ok(Self::filter_data(result_list))
    }

    async fn get_music_url(
        &self,
        music_info: &MusicInfo,
        quality: MusicQuality,
    ) -> Result<String> {
        // id 为 songId，播放接口需要 copyrightId（搜索时放在 quality map 中）
        let copyright_id = music_info.quality.get(&quality)
            .or_else(|| music_info.quality.values().next())
            .cloned()
            .unwrap_or_else(|| music_info.id.clone());
        let quality_code = match quality {
            MusicQuality::Lq => "PQ",
            MusicQuality::Mq => "HQ",
            MusicQuality::Hq => "HQ",
            MusicQuality::Sq => "SQ",
            MusicQuality::Hires => "ZQ",
        };

        let url = "https://app.c.nf.migu.cn/MIGU/3.0/content/sub/listenSongData.do";
        let params = [
            ("copyrightId", copyright_id.as_str()),
            ("resourceType", "2"),
            ("purpose", "1"),
            ("type", quality_code),
        ];

        let resp = self.client.get(url).query(&params).send().await?;
        let json: Value = resp.json().await?;

        if let Some(url) = json.get("data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .and_then(|f| f.get("url"))
            .and_then(|u| u.as_str())
            .filter(|s| !s.is_empty())
        {
            return Ok(url.to_string());
        }

        // 尝试备用接口
        Ok(format!("https://freetyst.nf.migu.cn/{}.mp3", copyright_id))
    }

    async fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let info = self.get_resource_info(&music_info.id).await?;
        let mrc_url = info.get("mrcUrl").and_then(|v| v.as_str()).unwrap_or("");
        let lrc_url = info.get("lrcUrl").and_then(|v| v.as_str()).unwrap_or("");
        let trc_url = info.get("trcUrl").and_then(|v| v.as_str()).unwrap_or("");

        let (lyric, lxlyric) = if !mrc_url.is_empty() {
            let text = self.get_text(mrc_url).await?;
            let decrypted = decrypt_mrc(&text)?;
            parse_mrc_lyric(&decrypted)
        } else if !lrc_url.is_empty() {
            (self.get_text(lrc_url).await?, String::new())
        } else {
            return Err(Error::Lyric("获取歌词失败".to_string()));
        };

        let tlyric = if trc_url.is_empty() {
            String::new()
        } else {
            self.get_text(trc_url).await?
        };

        Ok(LyricInfo {
            lyric,
            tlyric: if tlyric.is_empty() { None } else { Some(tlyric) },
            rlyric: None,
            lxlyric: if lxlyric.is_empty() { None } else { Some(lxlyric) },
        })
    }

    async fn get_pic_url(&self, music_info: &MusicInfo) -> Result<String> {
        if let Some(pic) = music_info.pic_url.as_ref().filter(|s| !s.is_empty()) {
            return Ok(pic.clone());
        }
        let info = self.get_resource_info(&music_info.id).await?;
        info.get("albumImgs")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|img| img.get("img"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::MusicSource("获取封面失败".to_string()))
    }

    async fn check_available(&self) -> Result<bool> {
        match self.client.get(&self.base_url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        search_sample: SearchSample,
        resource_info: ResourceInfo,
        mrc_hex: String,
        decrypted: String,
        parsed: Parsed,
        tlyric: String,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SearchSample {
        song_id: String,
        copyright_id: String,
        name: String,
        duration: u64,
        #[allow(dead_code)]
        img: String,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResourceInfo {
        length: String,
        #[allow(dead_code)]
        lrc_url: String,
        mrc_url: String,
        trc_url: String,
        album_img: String,
        formats: Vec<String>,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Parsed {
        lyric: String,
        lxlyric: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../../tests/fixtures/mg_mrc.json")).unwrap()
    }

    fn fixture_raw(f: &Fixture) -> String {
        String::from_utf8(crypto::hex_decode(&f.mrc_hex).unwrap()).unwrap()
    }

    #[test]
    fn test_create_signature() {
        assert_eq!(
            MiguSource::create_signature("1697000000000", "海阔天空"),
            "17f787cbc0e9ea9256f9f04907d51664"
        );
        assert_eq!(MiguSource::create_signature("1", "a").len(), 32);
    }

    #[test]
    fn test_encode_uri_component() {
        assert_eq!(
            MiguSource::encode_uri_component("海阔天空"),
            "%E6%B5%B7%E9%98%94%E5%A4%A9%E7%A9%BA"
        );
        assert_eq!(
            MiguSource::encode_uri_component("a b-c_d.e!~*'()"),
            "a%20b-c_d.e!~*'()"
        );
    }

    #[test]
    fn test_decrypt_mrc_fixture() {
        let f = fixture();
        let raw = fixture_raw(&f);
        assert_eq!(decrypt_mrc(&raw).unwrap(), f.decrypted);
    }

    #[test]
    fn test_parse_mrc_fixture() {
        let f = fixture();
        let raw = fixture_raw(&f);
        let decrypted = decrypt_mrc(&raw).unwrap();
        let (lyric, lxlyric) = parse_mrc_lyric(&decrypted);
        assert_eq!(lyric, f.parsed.lyric);
        assert_eq!(lxlyric, f.parsed.lxlyric);
    }

    #[test]
    fn test_fixture_shape() {
        let f = fixture();
        assert_eq!(f.search_sample.song_id, "1135162566");
        assert_eq!(f.search_sample.copyright_id, "6005752DXKE");
        assert_eq!(f.search_sample.duration, 239);
        assert_eq!(f.resource_info.length, "00:03:59");
        assert!(!f.resource_info.mrc_url.is_empty());
        assert!(f.resource_info.trc_url.is_empty());
        assert!(f.resource_info.formats.contains(&"SQ".to_string()));
        assert!(!f.resource_info.album_img.is_empty());
        assert!(f.tlyric.is_empty());
    }

    #[test]
    fn test_decrypt_mrc_short_passthrough() {
        assert_eq!(decrypt_mrc("short").unwrap(), "short");
    }

    #[test]
    fn test_parse_mrc_lyric_edge() {
        // ms 补 3 位、word start 早于行首时 trunc 出负数
        let (lyric, lxlyric) =
            parse_mrc_lyric("[1050,1000]abc(1050,300)def(900,100)\n[20050,0]hi\n");
        assert_eq!(lyric, "[00:01.050]abcdef\n[00:20.050]hi");
        assert_eq!(lxlyric, "[00:01.050]<0,300>abc<-150,100>def");
    }

    #[test]
    fn test_parse_mrc_lyric_skip_invalid_lines() {
        let (lyric, lxlyric) = parse_mrc_lyric("[ti:标题]\r\n \n[1000,0]ok(1000,100)x\n");
        assert_eq!(lyric, "[00:01.000]okx");
        assert_eq!(lxlyric, "[00:01.000]<0,100>ok");
    }

    #[test]
    fn test_filter_data() {
        let items: Value = serde_json::json!([
            [{
                "songId": "1135162566",
                "copyrightId": "6005752DXKE",
                "name": "海阔天空",
                "album": "乐与怒",
                "duration": 239,
                "singerList": [{ "name": "Beyond" }],
                "audioFormats": [
                    { "formatType": "PQ" },
                    { "formatType": "HQ" },
                    { "formatType": "SQ" },
                    { "formatType": "ZQ24" }
                ],
                "img3": "",
                "img2": "/data/oss/resource/00/2f/ya/bl.webp"
            }, {
                "songId": "1135162566",
                "copyrightId": "6005752DXKE",
                "name": "重复条目",
                "duration": 239,
                "audioFormats": []
            }, {
                "songId": "999",
                "copyrightId": "0009",
                "name": "无音质",
                "duration": 100,
                "singerList": [],
                "audioFormats": [],
                "img1": "https://img.example.com/a.jpg"
            }]
        ]);
        let list = MiguSource::filter_data(items.as_array().unwrap());
        assert_eq!(list.len(), 2);

        let m = &list[0];
        assert_eq!(m.id, "1135162566");
        assert_eq!(m.name, "海阔天空");
        assert_eq!(m.album_name, "乐与怒");
        assert_eq!(m.singer, vec!["Beyond".to_string()]);
        assert_eq!(m.interval, 239);
        assert_eq!(m.source, MusicSource::Mg);
        assert_eq!(
            m.pic_url.as_deref(),
            Some("http://d.musicapp.migu.cn/data/oss/resource/00/2f/ya/bl.webp")
        );
        assert_eq!(m.quality.len(), 4);
        assert_eq!(m.quality.get(&MusicQuality::Lq).map(String::as_str), Some("6005752DXKE"));
        assert_eq!(m.quality.get(&MusicQuality::Hq).map(String::as_str), Some("6005752DXKE"));
        assert_eq!(m.quality.get(&MusicQuality::Sq).map(String::as_str), Some("6005752DXKE"));
        assert_eq!(m.quality.get(&MusicQuality::Hires).map(String::as_str), Some("6005752DXKE"));

        let m2 = &list[1];
        assert_eq!(m2.id, "999");
        assert!(m2.quality.is_empty());
        assert_eq!(m2.pic_url.as_deref(), Some("https://img.example.com/a.jpg"));
    }

    #[test]
    fn test_filter_data_requires_ids() {
        let items: Value = serde_json::json!([
            [{ "name": "缺 songId", "copyrightId": "c1" }],
            [{ "songId": "s1", "name": "缺 copyrightId" }],
            []
        ]);
        let list = MiguSource::filter_data(items.as_array().unwrap());
        assert!(list.is_empty());
    }
}
