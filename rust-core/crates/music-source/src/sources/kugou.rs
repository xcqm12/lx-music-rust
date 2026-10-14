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

    /// 解码 KRC 歌词（与上游 kg/util.js decodeLyric 同口径：
    /// base64 -> 跳过 4 字节头 -> 16 字节密钥循环 XOR -> zlib inflate）
    fn decode_krc(&self, content: &str) -> Result<String> {
        let krc_bytes = crypto::base64_decode(content)
            .map_err(|e| Error::ParseFailed(e.to_string()))?;
        if krc_bytes.len() < 4 {
            return Err(Error::ParseFailed("krc too short".to_string()));
        }
        const ENC_KEY: [u8; 16] = [
            0x40, 0x47, 0x61, 0x77, 0x5e, 0x32, 0x74, 0x47,
            0x51, 0x36, 0x31, 0x2d, 0xce, 0xd2, 0x6e, 0x69,
        ];
        let data = &krc_bytes[4..];
        let xored: Vec<u8> = data.iter().enumerate()
            .map(|(i, b)| b ^ ENC_KEY[i % 16])
            .collect();

        let mut decoder = flate2::read::ZlibDecoder::new(&xored[..]);
        let mut result = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut result)
            .map_err(|e| Error::ParseFailed(e.to_string()))?;
        Ok(result)
    }

    /// 解析 KRC 歌词（与上游 kg/lyric.js parseLyric 同口径：
    /// 去头、提取 language 翻译 JSON、毫秒时间标签补位转换、逐字标签归一）
    fn parse_krc_lyric(&self, krc: &str) -> LyricInfo {
        let mut str_buf = krc.replace('\r', "");
        // 去掉 [id:$xxx] 头
        if let Ok(head_re) = regex::Regex::new(r"^.*\[id:\$\w+\]\n") {
            str_buf = head_re.replace(&str_buf, "").to_string();
        }

        // 提取 [language:...] 翻译（连同换行一起移除，与上游一致）
        let mut rlyric_lines: Option<Vec<String>> = None;
        let mut tlyric_lines: Option<Vec<String>> = None;
        if let Ok(lang_re) = regex::Regex::new(r"\[language:([\w=\\/+]+)\]") {
            if let Some(caps) = lang_re.captures(&str_buf) {
                let b64 = caps[1].to_string();
                str_buf = regex::Regex::new(r"\[language:[\w=\\/+]+\]\n")
                    .map(|re| re.replace(&str_buf, "").to_string())
                    .unwrap_or_else(|_| lang_re.replace(&str_buf, "").to_string());
                if let Ok(json_bytes) = crypto::base64_decode(&b64) {
                    if let Ok(json) = serde_json::from_slice::<Value>(&json_bytes) {
                        if let Some(content) = json.get("content").and_then(|c| c.as_array()) {
                            for item in content {
                                let lines = item.get("lyricContent")
                                    .and_then(|l| l.as_array())
                                    .map(|arr| {
                                        arr.iter().map(|line| match line {
                                            Value::Array(words) => words.iter()
                                                .filter_map(|w| w.as_str())
                                                .collect::<Vec<_>>()
                                                .join(""),
                                            Value::String(s) => s.clone(),
                                            _ => String::new(),
                                        }).collect::<Vec<_>>()
                                    });
                                match item.get("type").and_then(|t| t.as_i64()) {
                                    Some(0) => rlyric_lines = lines,
                                    Some(1) => tlyric_lines = lines,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        // 时间标签转换：[毫秒,时长] -> [mm:ss.mmm]（毫秒补 3 位，上游 1.9.1 修复点）
        // 注意：`.*` 在 Rust regex 中默认不跨 \n，与 JS 行内匹配语义一致
        let mut rlyric_out = rlyric_lines.clone();
        let mut tlyric_out = tlyric_lines.clone();
        let mut i = 0usize;
        let time_re = regex::Regex::new(r"\[((\d+),\d+)\].*").unwrap();
        let lxlyric = time_re.replace_all(&str_buf, |caps: &regex::Captures| {
            let ms: u64 = caps[2].parse().unwrap_or(0);
            let time_str = format!(
                "{:02}:{:02}.{:03}",
                ms / 60_000,
                (ms / 1000) % 60,
                ms % 1000
            );
            if let Some(lines) = rlyric_out.as_mut() {
                let text = lines.get(i).cloned().unwrap_or_default();
                if i < lines.len() {
                    lines[i] = format!("[{}]{}", time_str, text);
                } else {
                    lines.push(format!("[{}]{}", time_str, text));
                }
            }
            if let Some(lines) = tlyric_out.as_mut() {
                let text = lines.get(i).cloned().unwrap_or_default();
                if i < lines.len() {
                    lines[i] = format!("[{}]{}", time_str, text);
                } else {
                    lines.push(format!("[{}]{}", time_str, text));
                }
            }
            i += 1;
            // JS String.replace(string, ...) 只替换首个匹配
            caps[0].replacen(&caps[1], &time_str, 1)
        }).to_string();

        let rlyric = match rlyric_out {
            Some(lines) if !lines.is_empty() => utils::decode_html_entities(&lines.join("\n")),
            _ => String::new(),
        };
        let tlyric = match tlyric_out {
            Some(lines) if !lines.is_empty() => utils::decode_html_entities(&lines.join("\n")),
            _ => String::new(),
        };

        // 逐字歌词：<a,b,c> -> <a,b>（保留给 lxlyric），纯歌词再去掉 <a,b>
        let lxlyric = regex::Regex::new(r"<(\d+,\d+),\d+>")
            .map(|re| re.replace_all(&lxlyric, "<$1>").to_string())
            .unwrap_or(lxlyric);
        let lxlyric = utils::decode_html_entities(&lxlyric);
        let lyric = regex::Regex::new(r"<\d+,\d+>")
            .map(|re| re.replace_all(&lxlyric, "").to_string())
            .unwrap_or_else(|_| lxlyric.clone());

        LyricInfo {
            lyric,
            tlyric: if tlyric.is_empty() { None } else { Some(tlyric) },
            rlyric: if rlyric.is_empty() { None } else { Some(rlyric) },
            lxlyric: if lxlyric.is_empty() { None } else { Some(lxlyric) },
        }
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
        // 与上游 kg/musicSearch.js 同一接口（免签名）
        let url = "https://songsearch.kugou.com/song_search_v2";

        let resp = self.client
            .get(url)
            .query(&[
                ("keyword", keyword),
                ("page", &page.to_string()),
                ("pagesize", &limit.to_string()),
                ("userid", "0"),
                ("clientver", ""),
                ("platform", "WebFilter"),
                ("filter", "2"),
                ("iscorrection", "1"),
                ("privilege_filter", "0"),
                ("area_code", "1"),
            ])
            .send()
            .await?;

        let json: Value = resp.json().await?;

        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        if let Some(list) = json.get("data").and_then(|d| d.get("lists")).and_then(|l| l.as_array()) {
            for item in list {
                // 上游 handleResult：按 Audioid+FileHash 去重，并展开 Grp 分组
                let mut group: Vec<&Value> = vec![item];
                if let Some(grp) = item.get("Grp").and_then(|g| g.as_array()) {
                    group.extend(grp.iter());
                }
                for child in group {
                    let audioid = child.get("Audioid").map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    }).unwrap_or_default();
                    let key = format!(
                        "{}{}",
                        audioid,
                        child.get("FileHash").and_then(|v| v.as_str()).unwrap_or("")
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    if let Ok(music_info) = self.parse_music_info(child) {
                        results.push(music_info);
                    }
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
        // 与上游 kg/lyric.js getLyric 同链路：search -> download -> krc/lrc
        let search_url = "http://lyrics.kugou.com/search";
        let resp = self.client
            .get(search_url)
            .header("KG-RC", "1")
            .header("KG-THash", "expand_search_manager.cpp:852736169:451")
            .header("User-Agent", "KuGou2012-9020-ExpandSearchManager")
            .query(&[
                ("ver", "1"),
                ("man", "yes"),
                ("client", "pc"),
                ("keyword", music_info.name.as_str()),
                ("hash", music_info.id.as_str()),
                ("timelength", &music_info.interval.to_string()),
                ("lrctxt", "1"),
            ])
            .send()
            .await?;

        let json: Value = resp.json().await?;

        let candidate = json.get("candidates")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .ok_or_else(|| Error::MusicSource("Get lyric failed".to_string()))?;

        let id = candidate.get("id").and_then(|v| v.as_str())
            .ok_or_else(|| Error::MusicSource("Get lyric failed".to_string()))?;
        let access_key = candidate.get("accesskey").and_then(|v| v.as_str())
            .ok_or_else(|| Error::MusicSource("Get lyric failed".to_string()))?;
        // krctype == 1 且 contenttype != 1 时为 krc，否则 lrc
        let krctype = candidate.get("krctype").and_then(|v| v.as_i64()).unwrap_or(0);
        let contenttype = candidate.get("contenttype").and_then(|v| v.as_i64()).unwrap_or(0);
        let fmt = if krctype == 1 && contenttype != 1 { "krc" } else { "lrc" };

        let download_url = "http://lyrics.kugou.com/download";
        let dresp = self.client
            .get(download_url)
            .header("KG-RC", "1")
            .header("KG-THash", "expand_search_manager.cpp:852736169:451")
            .header("User-Agent", "KuGou2012-9020-ExpandSearchManager")
            .query(&[
                ("ver", "1"),
                ("client", "pc"),
                ("id", id),
                ("accesskey", access_key),
                ("fmt", fmt),
                ("charset", "utf8"),
            ])
            .send()
            .await?;

        let djson: Value = dresp.json().await?;
        let content = djson.get("content").and_then(|c| c.as_str())
            .ok_or_else(|| Error::MusicSource("Get lyric failed".to_string()))?;

        let body_fmt = djson.get("fmt").and_then(|f| f.as_str()).unwrap_or(fmt);
        match body_fmt {
            "krc" => {
                let krc_text = self.decode_krc(content)?;
                Ok(self.parse_krc_lyric(&krc_text))
            }
            "lrc" => {
                let lyric = crypto::base64_decode(content)
                    .map_err(|e| Error::ParseFailed(e.to_string()))
                    .and_then(|b| String::from_utf8(b).map_err(|e| Error::ParseFailed(e.to_string())))?;
                Ok(LyricInfo {
                    lyric,
                    tlyric: None,
                    rlyric: None,
                    lxlyric: None,
                })
            }
            _ => Err(Error::MusicSource(format!("未知歌词格式: {}", body_fmt))),
        }
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

        // 上游 1.9.1 搜索显示修复：OriSongName + Suffix（不再用 SongName，
        // 避免带修饰语的名字显示错误）
        let ori_name = item.get("OriSongName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let suffix = item.get("Suffix").and_then(|v| v.as_str()).unwrap_or("");
        let raw_name = if suffix.is_empty() {
            ori_name.to_string()
        } else {
            format!("{} {}", ori_name, suffix)
        };
        let name = utils::decode_html_entities(&raw_name);

        let singer = item.get("Singers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                    .map(|s| utils::decode_html_entities(s))
                    .collect()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
            .unwrap_or_else(|| {
                item.get("SingerName")
                    .and_then(|v| v.as_str())
                    .map(|s| utils::decode_html_entities(s)
                        .split('、')
                        .map(|s| s.trim().to_string())
                        .collect())
                    .unwrap_or_default()
            });

        let album_name = item.get("AlbumName")
            .and_then(|v| v.as_str())
            .map(|s| utils::decode_html_entities(s))
            .unwrap_or_default();

        let interval = item.get("Duration")
            .and_then(|v| v.as_i64())
            .map(|i| i as u32)
            .unwrap_or(0);

        let mut quality = std::collections::BTreeMap::new();

        // 与上游 filterData 同口径：按文件大小判断可用音质
        if item.get("HQFileSize").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
            if let Some(hq_hash) = item.get("HQFileHash").and_then(|v| v.as_str()) {
                if !hq_hash.is_empty() {
                    quality.insert(MusicQuality::Hq, hq_hash.to_string());
                }
            }
        }
        if item.get("SQFileSize").and_then(|v| v.as_i64()).unwrap_or(0) != 0 {
            if let Some(sq_hash) = item.get("SQFileHash").and_then(|v| v.as_str()) {
                if !sq_hash.is_empty() {
                    quality.insert(MusicQuality::Sq, sq_hash.to_string());
                }
            }
        }

        // 128k 始终尝试（FileSize 为 0 时退回 FileHash 亦无害）
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
