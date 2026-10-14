//! 酷我歌词链路，等价上游 src/utils/musicSdk/kw/{lyric.js,decodeLyric.js,util.js}
//! mlyric 二进制响应 → decodeLyric（inflate + 可选 yeelion XOR）→ parseLrc/sortLrcArr
//! → lrcTools.parse（逐字重排）→ existTimeExp 校验

use crate::crypto;
use common::{Error, LyricInfo, Result};
use once_cell::sync::Lazy;
use regex::Regex;

static LINE_SPLIT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\r\n|\r|\n").unwrap());
/// lyric.js timeExp（g 标志下因 ^ 锚定实际只命中/替换行首第一个）
static TIME_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\[([\d:.]*)\]").unwrap());
/// lyric.js tagLine（parseLrc 的 else-if 分支与 lrcTools 共用）
static TAG_LINE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"\[(ver|ti|ar|al|offset|by|kuwo):\s*(\S+(?:\s+\S+)*)\s*\]",
).unwrap());
/// lyric.js wordTimeAll strip 与 lyricxTag
static WORD_TIME_STRIP: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<(-?\d+),(-?\d+)(?:,-?\d+)?>").unwrap());
static LYRICX_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"^<-?\d+,-?\d+>").unwrap());
/// lyric.js existTimeExp
static EXIST_TIME: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\d{1,2}:.*\d{1,4}]").unwrap());
/// util.js lrcTools.rxps
static WORD_LINE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"^(\[\d{1,2}:.*\d{1,4}\])\s*(\S+(?:\s+\S+)*)?\s*",
).unwrap());
static WORD_TIME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<(-?\d+),(-?\d+)(?:,-?\d+)?>").unwrap());

const KW_XOR_KEY: &[u8] = b"yeelion";

// ===== decodeLyric.js =====

/// pako.inflate（zlib）等价；容忍尾部不完整流，输出为空才视为失败。
/// 注意 flate2 的 decompress_vec 只写 Vec 现有空闲容量不扩容，须用 io 层 ZlibDecoder
fn inflate_bytes(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let mut out = Vec::new();
    // 截断流 read_to_end 返回 Err，但已解码部分仍在 out 中，与 pako Z_SYNC_FLUSH 容忍口径一致
    let _ = ZlibDecoder::new(data).read_to_end(&mut out);
    if out.is_empty() {
        return Err(Error::LyricParseFailed("歌词 inflate 失败".to_string()));
    }
    Ok(out)
}

/// Node Buffer.from(str, 'base64') 宽松口径：忽略非 base64 字符、容忍缺省 padding
fn loose_base64_decode(s: &str) -> Vec<u8> {
    let filtered: String = s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '=')
        .collect();
    let trimmed = filtered.trim_end_matches('=');
    let pad = (4 - trimmed.len() % 4) % 4;
    let mut input = String::with_capacity(trimmed.len() + pad);
    input.push_str(trimmed);
    input.push_str(&"=".repeat(pad));
    crypto::base64_decode(&input).unwrap_or_default()
}

/// 同上游 decodeLyric：头 10 字节（utf8 + toLowerCase）须为 tp=content，
/// inflate 后非 lyricx 直接 utf8，lyricx 走 base64 + yeelion cycle XOR
pub fn decode_lyric(raw: &[u8], is_lyricx: bool) -> Result<String> {
    let head_end = raw.len().min(10);
    if String::from_utf8_lossy(&raw[..head_end]).to_lowercase() != "tp=content" {
        return Ok(String::new());
    }
    let sep = raw.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| i + 4)
        .unwrap_or(3)
        .min(raw.len());
    let inflated = inflate_bytes(&raw[sep..])?;
    if !is_lyricx {
        return Ok(String::from_utf8_lossy(&inflated).into_owned());
    }
    let b64_text = String::from_utf8_lossy(&inflated);
    let decoded = loose_base64_decode(&b64_text);
    let out: Vec<u8> = decoded.iter()
        .enumerate()
        .map(|(i, b)| b ^ KW_XOR_KEY[i % KW_XOR_KEY.len()])
        .collect();
    Ok(String::from_utf8_lossy(&out).into_owned())
}

// ===== he.decode 精简等价（歌词场景常用实体 + 数字实体）=====

fn entity_char(name: &str) -> Option<char> {
    if let Some(hex) = name.strip_prefix('#').and_then(|r| r.strip_prefix('x').or_else(|| r.strip_prefix('X'))) {
        return u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
    }
    if let Some(num) = name.strip_prefix('#') {
        return num.parse::<u32>().ok().and_then(char::from_u32);
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "times" => '×',
        "divide" => '÷',
        "middot" => '·',
        "bull" => '•',
        "deg" => '°',
        "plusmn" => '±',
        "sect" => '§',
        "para" => '¶',
        "dagger" => '†',
        _ => return None,
    })
}

fn decode_html_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        if let Some(end) = rest[..].find(';') {
            // is_char_boundary(1)：`&` 后若为多字节字符，rest[1..] 会 panic，
            // 此时保留原文（与 he.decode 找不到实体的行为一致）
            if (2..=12).contains(&end) && rest.is_char_boundary(1) {
                if let Some(c) = entity_char(&rest[1..end]) {
                    out.push(c);
                    rest = &rest[end + 1..];
                    continue;
                }
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

// ===== lyric.js parseLrc / sortLrcArr =====

struct LrcItem {
    time: String,
    text: String,
}

/// 上游 sortLrcArr：相同时间行视为翻译（挪到 lrcT 并改时间为上一原词行时间），
/// 无逐字标签且翻译比例异常时抛错（视为机翻误判）
fn sort_lrc_arr(arr: Vec<LrcItem>) -> Result<(Vec<LrcItem>, Vec<LrcItem>)> {
    let mut lrc_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut lrc: Vec<LrcItem> = Vec::new();
    let mut lrc_t: Vec<LrcItem> = Vec::new();
    let mut is_lyricx = false;

    for item in arr {
        if lrc_set.contains(&item.time) {
            if lrc.len() < 2 {
                continue;
            }
            let mut t_item = lrc.pop().unwrap();
            t_item.time = lrc[lrc.len() - 1].time.clone();
            if !is_lyricx && LYRICX_TAG.is_match(&item.text) {
                is_lyricx = true;
            }
            lrc_t.push(t_item);
            lrc.push(item);
        } else {
            if !is_lyricx && LYRICX_TAG.is_match(&item.text) {
                is_lyricx = true;
            }
            lrc_set.insert(item.time.clone());
            lrc.push(item);
        }
    }

    if !is_lyricx
        && (lrc_t.len() as f64) > (lrc.len() as f64) * 0.3
        && lrc.len().saturating_sub(lrc_t.len()) > 6
    {
        return Err(Error::LyricParseFailed("failed".to_string()));
    }
    Ok((lrc, lrc_t))
}

fn transform_lrc(tags: &[String], list: &[LrcItem]) -> String {
    let mut out = tags.join("\n");
    out.push('\n');
    for item in list {
        out.push_str(&format!("[{}]{}\n", item.time, item.text));
    }
    out
}

fn ends_with_dot2digits(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 3
        && b[b.len() - 3] == b'.'
        && b[b.len() - 2].is_ascii_digit()
        && b[b.len() - 1].is_ascii_digit()
}

fn parse_lrc(lrc: &str) -> Result<(String, String)> {
    let mut tags: Vec<String> = Vec::new();
    let mut lrc_arr: Vec<LrcItem> = Vec::new();

    for raw_line in LINE_SPLIT.split(lrc) {
        let line = raw_line.trim();
        if let Some(caps) = TIME_TAG.captures(line) {
            let text = TIME_TAG.replace(line, "").trim().to_string();
            let mut time = caps[1].to_string();
            if ends_with_dot2digits(&time) {
                time.push('0');
            }
            lrc_arr.push(LrcItem { time, text });
        } else if TAG_LINE.is_match(line) {
            tags.push(line.to_string());
        }
    }

    let (lrc_lines, lrc_t) = sort_lrc_arr(lrc_arr)?;
    let lyric = decode_html_entities(&transform_lrc(&tags, &lrc_lines));
    let tlyric = if lrc_t.is_empty() {
        String::new()
    } else {
        decode_html_entities(&transform_lrc(&tags, &lrc_t))
    };
    Ok((lyric, tlyric))
}

// ===== util.js lrcTools =====

/// JS parseInt 宽松前缀解析（radix 进制），无有效数字 → None（对应 NaN）
fn js_parse_int(s: &str, radix: u32) -> Option<i64> {
    let t = s.trim_start();
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let digits: String = t.chars().take_while(|c| c.to_digit(radix).is_some()).collect();
    if digits.is_empty() {
        return None;
    }
    let v = i64::from_str_radix(&digits, radix).ok()?;
    Some(if neg { -v } else { v })
}

struct WordInfo {
    start_time: i64,
    end_time: i64,
    time_str: String,
    new_time_str: Option<String>,
}

/// util.js getWordInfo：offset 字段为 (start,end) 绝对时间对，映射为逐字 <start,dur>
/// 分母取 abs 以覆盖负 offset（JS 侧走浮点 Math.abs）
fn get_word_info(a: i64, b: i64, offset: i64, offset2: i64, prev: Option<&mut WordInfo>) -> WordInfo {
    let start_time = (a + b).abs() / (offset * 2).abs();
    let end_time = start_time + (a - b).abs() / (offset2 * 2).abs();
    if let Some(p) = prev {
        if start_time < p.end_time {
            p.end_time = start_time;
            if p.start_time > p.end_time {
                p.start_time = p.end_time;
            }
            p.new_time_str = Some(format!("<{},{}>", p.start_time, p.end_time - p.start_time));
        }
    }
    WordInfo {
        start_time,
        end_time,
        time_str: format!("<{},{}>", start_time, end_time - start_time),
        new_time_str: None,
    }
}

struct LrcToolsState {
    offset: Option<i64>,
    offset2: Option<i64>,
    is_ok: bool,
    lines: Vec<String>,
    tags: Vec<String>,
}

fn parse_line(state: &mut LrcToolsState, line: &str) {
    if line.chars().count() < 6 {
        return;
    }
    if let Some(caps) = WORD_LINE.captures(line) {
        let time = caps[1].to_string();
        let mut words = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
        let word_times: Vec<String> = WORD_TIME.find_iter(&words)
            .map(|m| m.as_str().to_string())
            .collect();
        if word_times.is_empty() {
            return;
        }
        let mut prev: Option<WordInfo> = None;
        for time_str in word_times {
            let wc = match WORD_TIME.captures(&time_str) {
                Some(c) => c,
                None => continue,
            };
            let a: i64 = wc[1].parse().unwrap_or(0);
            let b: i64 = wc[2].parse().unwrap_or(0);
            let info = get_word_info(
                a,
                b,
                state.offset.unwrap_or(1),
                state.offset2.unwrap_or(1),
                prev.as_mut(),
            );
            words = words.replacen(&time_str, &info.time_str, 1);
            if let Some(p) = prev.as_ref() {
                if let Some(new_time) = &p.new_time_str {
                    let old = p.time_str.clone();
                    let new = new_time.clone();
                    words = words.replacen(&old, &new, 1);
                }
            }
            prev = Some(info);
        }
        state.lines.push(format!("{}{}", time, words));
        return;
    }
    if let Some(caps) = TAG_LINE.captures(line) {
        if &caps[1] == "kuwo" {
            let mut content = caps[2].to_string();
            if let Some(idx) = content.find("][") {
                content.truncate(idx);
            }
            // [kuwo:xxx] 八进制 → offset = value/10、offset2 = value%10，任一 0/NaN 则整首失败
            let value = js_parse_int(&content, 8);
            state.offset = value.map(|v| v / 10);
            state.offset2 = value.map(|v| v % 10);
            state.is_ok = matches!((state.offset, state.offset2), (Some(a), Some(b)) if a != 0 && b != 0);
        } else {
            state.tags.push(line.to_string());
        }
    }
}

/// util.js lrcTools.parse：逐字时间从 Kuwo 私有绝对时间重映射为 <start,dur>
fn lrc_tools_parse(lrc: &str) -> Result<String> {
    let mut state = LrcToolsState {
        offset: Some(1),
        offset2: Some(1),
        is_ok: true,
        lines: Vec::new(),
        tags: Vec::new(),
    };
    for line in LINE_SPLIT.split(lrc) {
        if !state.is_ok {
            return Err(Error::LyricParseFailed("failed".to_string()));
        }
        parse_line(&mut state, line);
    }
    if state.lines.is_empty() {
        return Ok(String::new());
    }
    let mut out = state.lines.join("\n");
    if !state.tags.is_empty() {
        out = format!("{}\n{}", state.tags.join("\n"), out);
    }
    Ok(out)
}

// ===== lyric.js getLyric promise 体 =====

/// 输入 decodeLyric 后的 LRC 文本，完成 parse → 翻译剥离 → 逐字生成 → 时间轴校验
pub fn get_lyric_info(lrc_text: &str) -> Result<LyricInfo> {
    let (lyric, mut tlyric) = parse_lrc(lrc_text)
        .map_err(|_| Error::Lyric("Get lyric failed".to_string()))?;
    if !tlyric.is_empty() {
        tlyric = WORD_TIME_STRIP.replace_all(&tlyric, "").into_owned();
    }
    let lxlyric = lrc_tools_parse(&lyric).unwrap_or_default();
    let lyric = WORD_TIME_STRIP.replace_all(&lyric, "").into_owned();
    if !EXIST_TIME.is_match(&lyric) {
        return Err(Error::Lyric("Get lyric failed".to_string()));
    }
    Ok(LyricInfo {
        lyric,
        tlyric: if tlyric.is_empty() { None } else { Some(tlyric) },
        rlyric: None,
        lxlyric: if lxlyric.is_empty() { None } else { Some(lxlyric) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        #[allow(dead_code)]
        rid: String,
        #[allow(dead_code)]
        url: String,
        raw_hex: String,
        decoded: String,
        parsed: Parsed,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Parsed {
        lyric: String,
        tlyric: String,
        lxlyric: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../tests/fixtures/kw_lyric.json")).unwrap()
    }

    #[test]
    fn test_decode_lyric_fixture() {
        let f = fixture();
        let raw = crypto::hex_decode(&f.raw_hex).unwrap();
        assert_eq!(decode_lyric(&raw, true).unwrap(), f.decoded);
    }

    #[test]
    fn test_get_lyric_info_fixture() {
        let f = fixture();
        let info = get_lyric_info(&f.decoded).unwrap();
        assert_eq!(info.lyric, f.parsed.lyric);
        assert_eq!(
            info.tlyric.as_deref(),
            if f.parsed.tlyric.is_empty() { None } else { Some(f.parsed.tlyric.as_str()) }
        );
        assert_eq!(
            info.lxlyric.as_deref(),
            if f.parsed.lxlyric.is_empty() { None } else { Some(f.parsed.lxlyric.as_str()) }
        );
    }

    #[test]
    fn test_decode_lyric_head_guard() {
        assert_eq!(decode_lyric(b"not a lyric payload", true).unwrap(), "");
        assert_eq!(decode_lyric(b"", true).unwrap(), "");
        // 头合法但正文 inflate 失败
        assert!(decode_lyric(b"tp=content", true).is_err());
    }

    #[test]
    fn test_decode_lyric_non_lyricx() {
        // tp=content 头 + zlib 压缩正文，非 lyricx 分支直接 utf8
        // （compress 用 io 层 ZlibEncoder：compress_vec 只写 Vec 空闲容量不扩容）
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;
        let body = "[00:01.000]hello";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut raw = b"tp=content\r\n\r\n".to_vec();
        raw.extend_from_slice(&compressed);
        assert_eq!(decode_lyric(&raw, false).unwrap(), body);
        // 同一 payload 走 lyricx 分支：inflate 后非 base64 文本，宽松解码后 XOR 得到确定结果
        let lyricx = decode_lyric(&raw, true).unwrap();
        assert!(!lyricx.is_empty());
    }

    #[test]
    fn test_sort_lrc_arr_trans_and_throw() {
        // 无逐字标签 + 翻译占比 > 0.3 且差值 > 6 → throw（12 原词 - 5 翻译 = 7 > 6）
        let arr: Vec<LrcItem> = (0..12)
            .map(|i| LrcItem {
                time: format!("00:0{:02}.000", i),
                text: "line".to_string(),
            })
            .chain((0..5).map(|i| LrcItem {
                time: format!("00:0{:02}.000", i),
                text: "译文".to_string(),
            }))
            .collect();
        assert!(sort_lrc_arr(arr).is_err());

        // 带逐字标签则放行
        let arr: Vec<LrcItem> = (0..10)
            .map(|i| LrcItem {
                time: format!("00:0{}.000", i),
                text: "<0,100>line".to_string(),
            })
            .chain((0..5).map(|i| LrcItem {
                time: format!("00:0{}.000", i),
                text: "译文".to_string(),
            }))
            .collect();
        let (lrc, lrc_t) = sort_lrc_arr(arr).unwrap();
        assert_eq!(lrc.len(), 10);
        assert_eq!(lrc_t.len(), 5);
        // 翻译行时间被改为 pop 前的上一原词行时间
        assert_eq!(lrc_t[0].time, "00:08.000");
        assert_eq!(lrc_t[1].time, "00:08.000");
    }

    #[test]
    fn test_lrc_tools_parse_basic() {
        // offset=1,offset2=1：词1 start=1000/2=500 end=500+1000/2=1000；
        // 词2 start=2000/2=1000 不小于 prev.end 无修正，end=1000+0
        let lrc = "[ti:标题]\n[00:01.000]<0,1000>ab<1000,1000>cd";
        let out = lrc_tools_parse(lrc).unwrap();
        assert_eq!(out, "[ti:标题]\n[00:01.000]<500,500>ab<1000,0>cd");
    }

    #[test]
    fn test_lrc_tools_parse_kuwo_offset() {
        // [kuwo:072] → parseInt('072',8)=58 → offset=5, offset2=8
        // 词1 start=1000/10=100 end=100+1000/16=162 → <100,62>
        // 词2 start=2000/10=200 不小于 162 → <200,0>
        let out = lrc_tools_parse("[kuwo:072]\n[00:01.000]<0,1000>ab<1000,1000>cd").unwrap();
        assert_eq!(out, "[00:01.000]<100,62>ab<200,0>cd");
        // 词2 start=(900+1000)/10=190 > prev.end 162 → 无修正 end=190+100/16=196
        let out2 = lrc_tools_parse("[kuwo:072]\n[00:01.000]<0,1000>ab<900,1000>cd").unwrap();
        assert_eq!(out2, "[00:01.000]<100,62>ab<190,6>cd");
    }

    #[test]
    fn test_lrc_tools_parse_kuwo_invalid() {
        // 非八进制内容 → offset NaN → isOK=false → 下一行抛错
        let lrc = "[kuwo:99]\n[00:01.000]<0,1000>ab";
        assert!(lrc_tools_parse(lrc).is_err());
        // kuwo 为最后一行时不检查（与上游一致）→ lines 为空返回 ''
        assert_eq!(lrc_tools_parse("[kuwo:99]").unwrap(), "");
    }

    #[test]
    fn test_get_word_info_prev_fix() {
        let mut prev = get_word_info(0, 1000, 1, 1, None);
        // start=|0+1000|/2=500, end=500+|0-1000|/2=1000
        assert_eq!(prev.time_str, "<500,500>");
        assert_eq!(prev.end_time, 1000);
        // 词2 start=|50+100|/2=75 < prev.end 1000 → prev 收敛为 [75,75]
        let next = get_word_info(50, 100, 1, 1, Some(&mut prev));
        assert_eq!(prev.time_str, "<500,500>");
        assert_eq!(prev.new_time_str.as_deref(), Some("<75,0>"));
        assert_eq!(next.time_str, "<75,25>");
    }

    #[test]
    fn test_parse_lrc_time_pad() {
        let (lyric, _) = parse_lrc("[ti:x]\n[00:01.50]hi\n").unwrap();
        // 两位小数补 3 位
        assert!(lyric.contains("[00:01.500]hi"));
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("a &amp; b &#65; &#x4E2D; &unknown;"), "a & b A 中 &unknown;");
        assert_eq!(decode_html_entities("无实体"), "无实体");
        assert_eq!(decode_html_entities("&#99999999999;"), "&#99999999999;");
    }
}
