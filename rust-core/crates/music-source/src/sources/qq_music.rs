use crate::{MusicSourceProvider, crypto, tx_lyric, utils};
use common::{Error, LyricInfo, MusicInfo, MusicQuality, MusicSource, Result};
use reqwest::Client;
use serde_json::{Value, json};
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

    /// GUID
    fn get_guid(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(1000000000_i64..9999999999_i64).to_string()
    }

    /// zzc 签名（与上游 tx/utils/crypto.js zzcSign 1:1）：
    /// SHA1 小写 hex -> 按固定索引取两段 -> 前 20 字节与 SCRAMBLE_VALUES 异或后
    /// base64 去 [\\/=+] -> 拼接并小写
    fn zzc_sign(text: &str) -> String {
        const PART_1_INDEXES: [usize; 8] = [23, 14, 6, 36, 16, 40, 7, 19];
        const PART_2_INDEXES: [usize; 8] = [16, 1, 32, 12, 19, 27, 8, 5];
        const SCRAMBLE_VALUES: [u8; 20] = [
            89, 39, 179, 150, 218, 82, 58, 252, 177, 52, 186, 123, 120, 64, 242, 133, 143, 161,
            121, 179,
        ];

        let hash = crypto::sha1_hex(text.as_bytes());
        let bytes = hash.as_bytes();
        // JS hash[idx] 越界得 undefined，join('') 时变空串 → Rust 用 get(i) 跳过
        let pick = |indexes: &[usize; 8]| -> String {
            indexes
                .iter()
                .filter_map(|&i| bytes.get(i))
                .map(|&b| b as char)
                .collect::<String>()
        };
        let part1 = pick(&PART_1_INDEXES);
        let part2 = pick(&PART_2_INDEXES);
        let part3: Vec<u8> = SCRAMBLE_VALUES
            .iter()
            .enumerate()
            .map(|(i, &value)| {
                let hash_byte = u8::from_str_radix(&hash[i * 2..i * 2 + 2], 16).unwrap_or(0);
                value ^ hash_byte
            })
            .collect();
        let b64_part = crypto::base64_encode(&part3)
            .chars()
            .filter(|c| !matches!(c, '\\' | '/' | '+' | '='))
            .collect::<String>();
        format!("zzc{}{}{}", part1, b64_part, part2).to_lowercase()
    }

    /// PC 客户端版 searchid：32 位大写十六进制 + 5 位补零随机数 = 37 字符
    fn get_search_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut guid = String::with_capacity(37);
        for _ in 0..32 {
            guid.push_str(&format!("{:X}", rng.gen_range(0u8..16)));
        }
        guid.push_str(&format!("{:05}", rng.gen_range(0u32..100_000)));
        guid
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
        // 与上游 tx/musicSearch.js 同一接口：zzcSign 签名 POST musics.fcg
        let req_data = json!({
            "comm": {
                "_channelid": "0",
                "_os_version": "6.2.9200-2",
                "ct": "19",
                "cv": "2151",
                "guid": "1F70E520B2EAA7D25E11760783C53CA9",
                "patch": "118",
                "psrf_access_token_expiresAt": 0,
                "psrf_qqaccess_token": "",
                "psrf_qqopenid": "",
                "psrf_qqunionid": "",
                "tmeAppID": "qqmusic",
                "tmeLoginType": 0,
                "uin": "0",
                "wid": "7223299733393904640",
            },
            "music.search.SearchCgiService": {
                "module": "music.search.SearchCgiService",
                "method": "DoSearchForQQMusicDesktop",
                "param": {
                    "grp": 1,
                    "num_per_page": limit,
                    "page_num": page,
                    "query": keyword,
                    "remoteplace": "txt.newclient.top",
                    "search_type": 0,
                    "searchid": Self::get_search_id(),
                },
            },
        });

        let sign = Self::zzc_sign(&req_data.to_string());
        let resp = self
            .client
            .post(format!("https://u.y.qq.com/cgi-bin/musics.fcg?sign={}", sign))
            .header("User-Agent", "QQMusic 14090508(android 12)")
            .json(&req_data)
            .send()
            .await?;

        let top: Value = resp.json().await?;
        // 与上游一致：取 music.search.SearchCgiService，缺失/为 null 时回退 req
        let req = match top.get("music.search.SearchCgiService") {
            Some(v) if !v.is_null() => v,
            _ => top
                .get("req")
                .ok_or_else(|| Error::MusicSource("搜索失败".to_string()))?,
        };
        if top.get("code").and_then(|c| c.as_i64()) != Some(0)
            || req.get("code").and_then(|c| c.as_i64()) != Some(0)
        {
            return Err(Error::MusicSource("搜索失败".to_string()));
        }

        let list = req
            .get("data")
            .and_then(|d| d.get("body"))
            .and_then(|b| b.get("song"))
            .and_then(|s| s.get("list"))
            .and_then(|l| l.as_array())
            .ok_or_else(|| Error::MusicSource("搜索失败".to_string()))?;

        let mut results = Vec::new();
        for item in list {
            if let Some(music_info) = self.handle_result(item) {
                results.push(music_info);
            }
        }
        Ok(results)
    }

    async fn get_music_url(
        &self,
        music_info: &MusicInfo,
        _quality: MusicQuality,
    ) -> Result<String> {
        let guid = self.get_guid();
        let uin = "0";

        let req_data = json!({
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

        let resp = self
            .client
            .get(url)
            .query(&[("data", req_data.to_string())])
            .send()
            .await?;

        let json: Value = resp.json().await?;

        if let Some(midurlinfo) = json
            .get("req_0")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.get("midurlinfo"))
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.first())
        {
            if let Some(purl) = midurlinfo.get("purl").and_then(|p| p.as_str()) {
                if !purl.is_empty() {
                    return Ok(format!("http://isure.stream.qqmusic.qq.com/{}", purl));
                }
            }
        }

        Err(Error::MusicSource("Failed to get music URL".to_string()))
    }

    async fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        // 与上游 tx/lyric.js getLyric 同链路：songId -> GetPlayLyricInfo -> QRC 解码 -> parse
        // 上游失败重试最多 3 次
        let mut last_err = Error::MusicSource("Get lyric failed".to_string());
        for _ in 0..4 {
            match self.get_lyric_once(music_info).await {
                Ok(info) => return Ok(info),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }

    async fn get_pic_url(&self, music_info: &MusicInfo) -> Result<String> {
        // 与上游 tx/index.js getPic 同口径（T002 albumMid），优先用搜索时构造的 pic_url
        if let Some(url) = &music_info.pic_url {
            if !url.is_empty() {
                return Ok(url.clone());
            }
        }
        let candidates = self.search(&music_info.name, 1, 10).await?;
        let target = utils::filter_str_lower(&music_info.name);
        for candidate in candidates {
            if utils::filter_str_lower(&candidate.name) == target
                && (candidate.interval as i32 - music_info.interval as i32).abs() <= 5
            {
                if let Some(url) = candidate.pic_url {
                    if !url.is_empty() {
                        return Ok(url);
                    }
                }
            }
        }
        Err(Error::MusicSource("Failed to get pic url".to_string()))
    }

    async fn check_available(&self) -> Result<bool> {
        match self.client.get("https://y.qq.com").send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl QQMusicSource {
    /// 单次获取歌词（含解码与解析）
    async fn get_lyric_once(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let song_id = self.get_song_id(music_info).await?;

        let body = json!({
            "comm": {
                "ct": "19",
                "cv": "1859",
                "uin": "0",
            },
            "req": {
                "method": "GetPlayLyricInfo",
                "module": "music.musichallSong.PlayLyricInfo",
                "param": {
                    "format": "json",
                    "crypt": 1,
                    "ct": 19,
                    "cv": 1873,
                    "interval": 0,
                    "lrc_t": 0,
                    "qrc": 1,
                    "qrc_t": 0,
                    "roma": 1,
                    "roma_t": 0,
                    "songID": song_id,
                    "trans": 1,
                    "trans_t": 0,
                    "type": -1,
                },
            },
        });

        let resp = self
            .client
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header("referer", "https://y.qq.com")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/86.0.4240.198 Safari/537.36",
            )
            .json(&body)
            .send()
            .await?;

        let json: Value = resp.json().await?;
        let req = json
            .get("req")
            .ok_or_else(|| Error::MusicSource("Get lyric failed".to_string()))?;
        if json.get("code").and_then(|c| c.as_i64()) != Some(0)
            || req.get("code").and_then(|c| c.as_i64()) != Some(0)
        {
            return Err(Error::MusicSource("Get lyric failed".to_string()));
        }
        let data = req.get("data").ok_or_else(|| {
            Error::MusicSource("Get lyric failed".to_string())
        })?;

        let get_hex = |key: &str| -> String {
            data.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
        };
        let lrc = tx_lyric::decode_qrc(&get_hex("lyric"))?;
        let tlrc = tx_lyric::decode_qrc(&get_hex("trans"))?;
        let rlrc = tx_lyric::decode_qrc(&get_hex("roma"))?;

        Ok(tx_lyric::parse(&lrc, &tlrc, &rlrc))
    }

    /// 上游 getSongId：songId（纯数字 id）直用，否则按 songmid 查 track_info.id
    async fn get_song_id(&self, music_info: &MusicInfo) -> Result<Value> {
        if let Ok(id) = music_info.id.parse::<i64>() {
            return Ok(json!(id));
        }

        let body = json!({
            "comm": {
                "ct": "19",
                "cv": "1859",
                "uin": "0",
            },
            "req": {
                "module": "music.pf_song_detail_svr",
                "method": "get_song_detail_yqq",
                "param": {
                    "song_type": 0,
                    "song_mid": music_info.id,
                },
            },
        });

        let resp = self
            .client
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header(
                "User-Agent",
                "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)",
            )
            .json(&body)
            .send()
            .await?;

        let json: Value = resp.json().await?;
        let req = json
            .get("req")
            .ok_or_else(|| Error::MusicSource("获取歌曲信息失败".to_string()))?;
        if json.get("code").and_then(|c| c.as_i64()) != Some(0)
            || req.get("code").and_then(|c| c.as_i64()) != Some(0)
        {
            return Err(Error::MusicSource("获取歌曲信息失败".to_string()));
        }
        let track_info = req
            .get("data")
            .and_then(|d| d.get("track_info"))
            .ok_or_else(|| Error::MusicSource("获取歌曲信息失败".to_string()))?;
        match track_info.get("id") {
            Some(Value::Number(n)) => Ok(Value::Number(n.clone())),
            Some(Value::String(s)) => Ok(json!(s)),
            _ => Err(Error::MusicSource("获取歌曲信息失败".to_string())),
        }
    }

    /// 搜索结果映射（与上游 musicSearch.js handleResult 同口径）
    fn handle_result(&self, item: &Value) -> Option<MusicInfo> {
        let file = item.get("file")?;
        if file.get("media_mid").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            return None;
        }

        let mid = item.get("mid").and_then(|v| v.as_str())?.to_string();
        let name = item.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let singer: Vec<String> = item
            .get("singer")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let album = item.get("album");
        let album_mid = album
            .and_then(|a| a.get("mid"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let album_name = album
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let interval = item
            .get("interval")
            .and_then(|v| v.as_i64())
            .map(|i| i as u32)
            .unwrap_or(0);

        let size_of = |key: &str| -> i64 {
            file.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
        };
        let mut quality = std::collections::BTreeMap::new();
        if size_of("size_128mp3") != 0 {
            quality.insert(MusicQuality::Lq, mid.clone());
        }
        if size_of("size_320mp3") != 0 {
            quality.insert(MusicQuality::Hq, mid.clone());
        }
        if size_of("size_flac") != 0 {
            quality.insert(MusicQuality::Sq, mid.clone());
        }
        if size_of("size_hires") != 0 {
            quality.insert(MusicQuality::Hires, mid.clone());
        }

        // 与上游 img 同规则：albumMid 有值用 T002，否则首个歌手 T001
        let pic_url = if album_mid.is_empty() || album_mid == "空" {
            match singer.first().and_then(|_| {
                item.get("singer")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|s| s.get("mid"))
                    .and_then(|v| v.as_str())
            }) {
                Some(singer_mid) => Some(format!(
                    "https://y.gtimg.cn/music/photo_new/T001R500x500M000{}.jpg",
                    singer_mid
                )),
                None => None,
            }
        } else {
            Some(format!(
                "https://y.gtimg.cn/music/photo_new/T002R500x500M000{}.jpg",
                album_mid
            ))
        };

        Some(MusicInfo {
            id: mid,
            name,
            singer,
            album_name,
            interval,
            source: MusicSource::Tx,
            quality,
            pic_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zzc_sign_fixed_vectors() {
        assert_eq!(
            QQMusicSource::zzc_sign("{\"a\":1}"),
            "zzc7746109xq501htmv4ipz7c8owlxfpihjm1fd695c7"
        );
        assert_eq!(
            QQMusicSource::zzc_sign("海阔天空"),
            "zzc9ae3008czrdhiyofg8hcyybwb2utbkqq002b086c8"
        );
    }

    #[test]
    fn test_zzc_sign_shape() {
        let sign = QQMusicSource::zzc_sign("test");
        assert!(sign.starts_with("zzc"));
        assert!(sign.len() >= 3 + 8 + 16 + 8);
        assert_eq!(sign, sign.to_lowercase());
        // 不含 base64 特殊字符
        assert!(!sign.contains(['/', '+', '=']));
    }

    #[test]
    fn test_get_search_id() {
        let id = QQMusicSource::get_search_id();
        assert_eq!(id.len(), 37);
        assert!(id[..32]
            .chars()
            .all(|c| (c.is_ascii_hexdigit() && c.is_ascii_uppercase()) || c.is_ascii_digit()));
        assert!(id[32..].chars().all(|c| c.is_ascii_digit()));
    }
}
