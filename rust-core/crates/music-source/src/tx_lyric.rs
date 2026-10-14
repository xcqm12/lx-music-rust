//! 腾讯 QRC 歌词解码与解析（与上游 tx/qrcDecode.js、tx/lyric.js parseTools 1:1 等价移植）
//!
//! 数据流：hex -> 3DES-ECB 解密（非标 S-box）-> zlib inflate（Z_SYNC_FLUSH 容忍尾部
//! 不完整流）-> UTF-8 -> parseTools 解析为 LyricInfo。

use common::{LyricInfo, Result};

// ===== 3DES（源自原 C++ native 插件，经 qrcDecode.js 移植） =====

const DES_ENCRYPT: bool = false;
const DES_DECRYPT: bool = true;

// QRC 使用的是非标准 DES 变体：S2[23]=15、S4[53]=10 与标准 DES 不同（标准 14、1）。
// 不可改为标准值，否则解密结果错误。下表逐字节对应原实现。
#[rustfmt::skip]
const SBOX: [[u32; 64]; 8] = [
    [14,4,13,1,2,15,11,8,3,10,6,12,5,9,0,7,0,15,7,4,14,2,13,1,10,6,12,11,9,5,3,8,4,1,14,8,13,6,2,11,15,12,9,7,3,10,5,0,15,12,8,2,4,9,1,7,5,11,3,14,10,0,6,13],
    [15,1,8,14,6,11,3,4,9,7,2,13,12,0,5,10,3,13,4,7,15,2,8,15,12,0,1,10,6,9,11,5,0,14,7,11,10,4,13,1,5,8,12,6,9,3,2,15,13,8,10,1,3,15,4,2,11,6,7,12,0,5,14,9],
    [10,0,9,14,6,3,15,5,1,13,12,7,11,4,2,8,13,7,0,9,3,4,6,10,2,8,5,14,12,11,15,1,13,6,4,9,8,15,3,0,11,1,2,12,5,10,14,7,1,10,13,0,6,9,8,7,4,15,14,3,11,5,2,12],
    [7,13,14,3,0,6,9,10,1,2,8,5,11,12,4,15,13,8,11,5,6,15,0,3,4,7,2,12,1,10,14,9,10,6,9,0,12,11,7,13,15,1,3,14,5,2,8,4,3,15,0,6,10,10,13,8,9,4,5,11,12,7,2,14],
    [2,12,4,1,7,10,11,6,8,5,3,15,13,0,14,9,14,11,2,12,4,7,13,1,5,0,15,10,3,9,8,6,4,2,1,11,10,13,7,8,15,9,12,5,6,3,0,14,11,8,12,7,1,14,2,13,6,15,0,9,10,4,5,3],
    [12,1,10,15,9,2,6,8,0,13,3,4,14,7,5,11,10,15,4,2,7,12,9,5,6,1,13,14,0,11,3,8,9,14,15,5,2,8,12,3,7,0,4,10,1,13,11,6,4,3,2,12,9,5,15,10,11,14,1,7,6,0,8,13],
    [4,11,2,14,15,0,8,13,3,12,9,7,5,10,6,1,13,0,11,7,4,9,1,10,14,3,5,12,2,15,8,6,1,4,11,13,12,3,7,14,10,15,6,8,0,5,9,2,6,11,13,8,1,4,10,7,9,5,0,15,14,2,3,12],
    [13,2,8,4,6,15,11,1,10,9,3,14,5,0,12,7,1,15,13,8,10,3,7,4,12,5,6,11,0,14,9,2,7,11,4,1,9,12,14,2,0,6,10,13,15,3,5,8,2,1,14,7,4,10,8,13,15,12,9,0,3,5,6,11],
];

const KEY_RND_SHIFT: [u32; 16] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];
const KEY_PERM_C: [u32; 28] = [
    56, 48, 40, 32, 24, 16, 8, 0, 57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18, 10, 2,
    59, 51, 43, 35,
];
const KEY_PERM_D: [u32; 28] = [
    62, 54, 46, 38, 30, 22, 14, 6, 61, 53, 45, 37, 29, 21, 13, 5, 60, 52, 44, 36, 28, 20, 12, 4,
    27, 19, 11, 3,
];
const KEY_COMPRESSION: [u32; 48] = [
    13, 16, 10, 23, 0, 4, 2, 27, 14, 5, 20, 9, 22, 18, 11, 3, 25, 7, 15, 6, 26, 19, 12, 1, 40,
    51, 30, 36, 46, 54, 29, 39, 50, 44, 32, 47, 43, 48, 38, 55, 33, 52, 45, 41, 49, 35, 28, 31,
];

/// 从 8 字节（大端位编号 0..63）取第 b 位放到第 c 位
fn bitnum(a: &[u8], b: u32, c: u32) -> u32 {
    let idx = (b / 32) * 4 + 3 - ((b % 32) / 8);
    (((a[idx as usize] >> (7 - (b % 8))) & 1) as u32) << c
}

/// 从 32 位（大端位编号 0..31）取第 b 位放到第 c 位
fn bitnum_intr(a: u32, b: u32, c: u32) -> u32 {
    ((a >> (31 - b)) & 1) << c
}

/// 从 32 位（小端位编号 0..31，b=0 为最低位）取位放到第 c 位
fn bitnum_intl(a: u32, b: u32, c: u32) -> u32 {
    ((a << b) & 0x80000000) >> c
}

fn sbox_bit(a: u32) -> usize {
    ((a & 32) | ((a & 31) >> 1) | ((a & 1) << 4)) as usize
}

fn initial_permutation(input: &[u8]) -> (u32, u32) {
    let s0 = bitnum(input, 57, 31) | bitnum(input, 49, 30) | bitnum(input, 41, 29)
        | bitnum(input, 33, 28) | bitnum(input, 25, 27) | bitnum(input, 17, 26)
        | bitnum(input, 9, 25) | bitnum(input, 1, 24) | bitnum(input, 59, 23)
        | bitnum(input, 51, 22) | bitnum(input, 43, 21) | bitnum(input, 35, 20)
        | bitnum(input, 27, 19) | bitnum(input, 19, 18) | bitnum(input, 11, 17)
        | bitnum(input, 3, 16) | bitnum(input, 61, 15) | bitnum(input, 53, 14)
        | bitnum(input, 45, 13) | bitnum(input, 37, 12) | bitnum(input, 29, 11)
        | bitnum(input, 21, 10) | bitnum(input, 13, 9) | bitnum(input, 5, 8)
        | bitnum(input, 63, 7) | bitnum(input, 55, 6) | bitnum(input, 47, 5)
        | bitnum(input, 39, 4) | bitnum(input, 31, 3) | bitnum(input, 23, 2)
        | bitnum(input, 15, 1) | bitnum(input, 7, 0);
    let s1 = bitnum(input, 56, 31) | bitnum(input, 48, 30) | bitnum(input, 40, 29)
        | bitnum(input, 32, 28) | bitnum(input, 24, 27) | bitnum(input, 16, 26)
        | bitnum(input, 8, 25) | bitnum(input, 0, 24) | bitnum(input, 58, 23)
        | bitnum(input, 50, 22) | bitnum(input, 42, 21) | bitnum(input, 34, 20)
        | bitnum(input, 26, 19) | bitnum(input, 18, 18) | bitnum(input, 10, 17)
        | bitnum(input, 2, 16) | bitnum(input, 60, 15) | bitnum(input, 52, 14)
        | bitnum(input, 44, 13) | bitnum(input, 36, 12) | bitnum(input, 28, 11)
        | bitnum(input, 20, 10) | bitnum(input, 12, 9) | bitnum(input, 4, 8)
        | bitnum(input, 62, 7) | bitnum(input, 54, 6) | bitnum(input, 46, 5)
        | bitnum(input, 38, 4) | bitnum(input, 30, 3) | bitnum(input, 22, 2)
        | bitnum(input, 14, 1) | bitnum(input, 6, 0);
    (s0, s1)
}

fn inverse_permutation(s0: u32, s1: u32, out: &mut [u8]) {
    out[3] = (bitnum_intr(s1, 7, 7) | bitnum_intr(s0, 7, 6) | bitnum_intr(s1, 15, 5)
        | bitnum_intr(s0, 15, 4) | bitnum_intr(s1, 23, 3) | bitnum_intr(s0, 23, 2)
        | bitnum_intr(s1, 31, 1) | bitnum_intr(s0, 31, 0)) as u8;
    out[2] = (bitnum_intr(s1, 6, 7) | bitnum_intr(s0, 6, 6) | bitnum_intr(s1, 14, 5)
        | bitnum_intr(s0, 14, 4) | bitnum_intr(s1, 22, 3) | bitnum_intr(s0, 22, 2)
        | bitnum_intr(s1, 30, 1) | bitnum_intr(s0, 30, 0)) as u8;
    out[1] = (bitnum_intr(s1, 5, 7) | bitnum_intr(s0, 5, 6) | bitnum_intr(s1, 13, 5)
        | bitnum_intr(s0, 13, 4) | bitnum_intr(s1, 21, 3) | bitnum_intr(s0, 21, 2)
        | bitnum_intr(s1, 29, 1) | bitnum_intr(s0, 29, 0)) as u8;
    out[0] = (bitnum_intr(s1, 4, 7) | bitnum_intr(s0, 4, 6) | bitnum_intr(s1, 12, 5)
        | bitnum_intr(s0, 12, 4) | bitnum_intr(s1, 20, 3) | bitnum_intr(s0, 20, 2)
        | bitnum_intr(s1, 28, 1) | bitnum_intr(s0, 28, 0)) as u8;
    out[7] = (bitnum_intr(s1, 3, 7) | bitnum_intr(s0, 3, 6) | bitnum_intr(s1, 11, 5)
        | bitnum_intr(s0, 11, 4) | bitnum_intr(s1, 19, 3) | bitnum_intr(s0, 19, 2)
        | bitnum_intr(s1, 27, 1) | bitnum_intr(s0, 27, 0)) as u8;
    out[6] = (bitnum_intr(s1, 2, 7) | bitnum_intr(s0, 2, 6) | bitnum_intr(s1, 10, 5)
        | bitnum_intr(s0, 10, 4) | bitnum_intr(s1, 18, 3) | bitnum_intr(s0, 18, 2)
        | bitnum_intr(s1, 26, 1) | bitnum_intr(s0, 26, 0)) as u8;
    out[5] = (bitnum_intr(s1, 1, 7) | bitnum_intr(s0, 1, 6) | bitnum_intr(s1, 9, 5)
        | bitnum_intr(s0, 9, 4) | bitnum_intr(s1, 17, 3) | bitnum_intr(s0, 17, 2)
        | bitnum_intr(s1, 25, 1) | bitnum_intr(s0, 25, 0)) as u8;
    out[4] = (bitnum_intr(s1, 0, 7) | bitnum_intr(s0, 0, 6) | bitnum_intr(s1, 8, 5)
        | bitnum_intr(s0, 8, 4) | bitnum_intr(s1, 16, 3) | bitnum_intr(s0, 16, 2)
        | bitnum_intr(s1, 24, 1) | bitnum_intr(s0, 24, 0)) as u8;
}

fn des_f(state: u32, key: &[u8]) -> u32 {
    let t1 = bitnum_intl(state, 31, 0) | ((state & 0xF0000000) >> 1) | bitnum_intl(state, 4, 5)
        | bitnum_intl(state, 3, 6) | ((state & 0x0F000000) >> 3) | bitnum_intl(state, 8, 11)
        | bitnum_intl(state, 7, 12) | ((state & 0x00F00000) >> 5) | bitnum_intl(state, 12, 17)
        | bitnum_intl(state, 11, 18) | ((state & 0x000F0000) >> 7) | bitnum_intl(state, 16, 23);
    let t2 = bitnum_intl(state, 15, 0) | ((state & 0x0000F000) << 15) | bitnum_intl(state, 20, 5)
        | bitnum_intl(state, 19, 6) | ((state & 0x00000F00) << 13) | bitnum_intl(state, 24, 11)
        | bitnum_intl(state, 23, 12) | ((state & 0x000000F0) << 11) | bitnum_intl(state, 28, 17)
        | bitnum_intl(state, 27, 18) | ((state & 0x0000000F) << 9) | bitnum_intl(state, 0, 23);

    let mut lrgstate = [
        (t1 >> 24) & 0xFF,
        (t1 >> 16) & 0xFF,
        (t1 >> 8) & 0xFF,
        (t2 >> 24) & 0xFF,
        (t2 >> 16) & 0xFF,
        (t2 >> 8) & 0xFF,
    ];
    for (i, b) in lrgstate.iter_mut().enumerate() {
        *b ^= key[i] as u32;
    }

    let s = (SBOX[0][sbox_bit(lrgstate[0] >> 2)] << 28)
        | (SBOX[1][sbox_bit(((lrgstate[0] & 0x03) << 4) | (lrgstate[1] >> 4))] << 24)
        | (SBOX[2][sbox_bit(((lrgstate[1] & 0x0F) << 2) | (lrgstate[2] >> 6))] << 20)
        | (SBOX[3][sbox_bit(lrgstate[2] & 0x3F)] << 16)
        | (SBOX[4][sbox_bit(lrgstate[3] >> 2)] << 12)
        | (SBOX[5][sbox_bit(((lrgstate[3] & 0x03) << 4) | (lrgstate[4] >> 4))] << 8)
        | (SBOX[6][sbox_bit(((lrgstate[4] & 0x0F) << 2) | (lrgstate[5] >> 6))] << 4)
        | SBOX[7][sbox_bit(lrgstate[5] & 0x3F)];

    bitnum_intl(s, 15, 0) | bitnum_intl(s, 6, 1) | bitnum_intl(s, 19, 2) | bitnum_intl(s, 20, 3)
        | bitnum_intl(s, 28, 4) | bitnum_intl(s, 11, 5) | bitnum_intl(s, 27, 6)
        | bitnum_intl(s, 16, 7) | bitnum_intl(s, 0, 8) | bitnum_intl(s, 14, 9)
        | bitnum_intl(s, 22, 10) | bitnum_intl(s, 25, 11) | bitnum_intl(s, 4, 12)
        | bitnum_intl(s, 17, 13) | bitnum_intl(s, 30, 14) | bitnum_intl(s, 9, 15)
        | bitnum_intl(s, 1, 16) | bitnum_intl(s, 7, 17) | bitnum_intl(s, 23, 18)
        | bitnum_intl(s, 13, 19) | bitnum_intl(s, 31, 20) | bitnum_intl(s, 26, 21)
        | bitnum_intl(s, 2, 22) | bitnum_intl(s, 8, 23) | bitnum_intl(s, 18, 24)
        | bitnum_intl(s, 12, 25) | bitnum_intl(s, 29, 26) | bitnum_intl(s, 5, 27)
        | bitnum_intl(s, 21, 28) | bitnum_intl(s, 10, 29) | bitnum_intl(s, 3, 30)
        | bitnum_intl(s, 24, 31)
}

fn des_crypt(input: &[u8], schedule: &[[u8; 6]; 16], output: &mut [u8]) {
    let (mut s0, mut s1) = initial_permutation(input);
    for item in schedule.iter().take(15) {
        let prev = s1;
        s1 = des_f(s1, item) ^ s0;
        s0 = prev;
    }
    s0 = des_f(s1, &schedule[15]) ^ s0;
    inverse_permutation(s0, s1, output);
}

fn key_schedule(key: &[u8], mode: bool) -> [[u8; 6]; 16] {
    let mut schedule = [[0u8; 6]; 16];
    let mut c: u32 = 0;
    let mut d: u32 = 0;
    for i in 0..28u32 {
        c |= bitnum(key, KEY_PERM_C[i as usize], 31 - i);
        d |= bitnum(key, KEY_PERM_D[i as usize], 31 - i);
    }
    for i in 0..16usize {
        let shift = KEY_RND_SHIFT[i];
        c = ((c << shift) | (c >> (28 - shift))) & 0xFFFFFFF0;
        d = ((d << shift) | (d >> (28 - shift))) & 0xFFFFFFF0;
        let togen = if mode == DES_DECRYPT { 15 - i } else { i };
        for j in 0..24usize {
            schedule[togen][j / 8] |= bitnum_intr(c, KEY_COMPRESSION[j], (7 - (j % 8)) as u32) as u8;
        }
        for j in 24..48usize {
            schedule[togen][j / 8] |= bitnum_intr(d, KEY_COMPRESSION[j] - 27, (7 - (j % 8)) as u32) as u8;
        }
    }
    schedule
}

type DesSchedule = [[u8; 6]; 16];
type TripleDesSchedule = [DesSchedule; 3];

fn tripledes_key_setup(key: &[u8], mode: bool) -> TripleDesSchedule {
    if mode == DES_ENCRYPT {
        [
            key_schedule(&key[0..8], DES_ENCRYPT),
            key_schedule(&key[8..16], DES_DECRYPT),
            key_schedule(&key[16..24], DES_ENCRYPT),
        ]
    } else {
        [
            key_schedule(&key[16..24], DES_DECRYPT),
            key_schedule(&key[8..16], DES_ENCRYPT),
            key_schedule(&key[0..8], DES_DECRYPT),
        ]
    }
}

fn tripledes_crypt(input: &[u8], schedule: &TripleDesSchedule, output: &mut [u8]) {
    let mut buf = [0u8; 8];
    des_crypt(input, &schedule[0], &mut buf);
    des_crypt(&buf, &schedule[1], output);
    des_crypt(output, &schedule[2], &mut buf);
    output.copy_from_slice(&buf);
}

// QRC 固定密钥：!@#)(*$%123ZXC!@!@#)(NHL
const QRC_KEY: [u8; 24] = [
    0x21, 0x40, 0x23, 0x29, 0x28, 0x2a, 0x24, 0x25, 0x31, 0x32, 0x33, 0x5a, 0x58, 0x43, 0x21,
    0x40, 0x21, 0x40, 0x23, 0x29, 0x28, 0x4e, 0x48, 0x4c,
];

/// 解密腾讯 QRC 歌词：hex -> 3DES-ECB -> zlib inflate -> UTF-8。
/// 解析失败时返回空字符串（与上游 decodeQrc 返回 '' 口径一致）。
pub fn decode_qrc(hex_data: &str) -> Result<String> {
    if hex_data.is_empty() || hex_data.len() % 2 != 0 {
        return Ok(String::new());
    }
    let mut encrypted = match hex::decode(hex_data) {
        Ok(v) => v,
        Err(_) => return Ok(String::new()),
    };
    if encrypted.is_empty() {
        return Ok(String::new());
    }

    let schedule = tripledes_key_setup(&QRC_KEY, DES_DECRYPT);
    for block in encrypted.chunks_exact_mut(8) {
        let mut input = [0u8; 8];
        input.copy_from_slice(block);
        let mut output = [0u8; 8];
        tripledes_crypt(&input, &schedule, &mut output);
        block.copy_from_slice(&output);
    }

    // 截断流 read_to_end 返回 Err，但已解码部分仍在 out 中，与 pako finishFlush: Z_SYNC_FLUSH 口径一致
    // （flate2 的 decompress_vec 只写 Vec 空闲容量不扩容，须用 io 层 ZlibDecoder）
    use std::io::Read;
    let mut out = Vec::new();
    let _ = flate2::read::ZlibDecoder::new(&encrypted[..]).read_to_end(&mut out);

    match String::from_utf8(out) {
        Ok(s) => Ok(s),
        Err(_) => Ok(String::new()),
    }
}

// ===== parseTools（tx/lyric.js 1:1） =====

fn line_time_re() -> regex::Regex {
    regex::Regex::new(r"^\[(\d+),\d+\]").unwrap()
}

fn line_time2_re() -> regex::Regex {
    regex::Regex::new(r"^\[([\d:.]+)\]").unwrap()
}

fn word_time_re() -> regex::Regex {
    regex::Regex::new(r"\(\d+,\d+\)").unwrap()
}

/// `[毫秒]` -> `[mm:ss.mmm]`（NaN 分支不适用：Rust 侧输入恒为整数）
fn ms_format(time_ms: i64) -> String {
    let ms = time_ms % 1000;
    let total_s = time_ms / 1000;
    let m = total_s / 60;
    let s = total_s % 60;
    format!("[{:02}:{:02}.{:03}]", m, s, ms)
}

/// 剥离 QRC XML 包装，取 LyricContent 属性值
fn remove_tag(str: &str) -> String {
    let re1 = regex::Regex::new(r#"(?s)^.*?LyricContent=""#).unwrap();
    let s = re1.replace(str, "");
    let re2 = regex::Regex::new(r#"(?s)"/>.*$"#).unwrap();
    re2.replace(&s, "").to_string()
}

/// `hh:mm:ss.mmm` / `mm:ss.mmm` / `m.s` -> 毫秒
fn get_intv(interval: &str) -> i64 {
    if interval.is_empty() {
        return 0;
    }
    let mut interval = interval.to_string();
    if !interval.contains('.') {
        interval.push_str(".0");
    }
    let re = regex::Regex::new(r"[:.]").unwrap();
    let mut arr: Vec<&str> = re.split(&interval).collect();
    while arr.len() < 3 {
        arr.insert(0, "0");
    }
    let parse = |v: &str| v.parse::<i64>().unwrap_or(0);
    parse(arr[0]) * 3_600_000 + parse(arr[1]) * 1000 + parse(arr[2])
}

fn parse_lyric(lrc: &str) -> (String, String) {
    let lrc = lrc.trim().replace('\r', "");
    if lrc.is_empty() {
        return (String::new(), String::new());
    }
    let line_time = line_time_re();
    let line_time2 = line_time2_re();
    let word_time = word_time_re();

    let mut lxlrc_lines: Vec<String> = Vec::new();
    let mut lrc_lines: Vec<String> = Vec::new();

    for line in lrc.split('\n') {
        let line = line.trim();
        let Some(caps) = line_time.captures(line) else {
            if line.starts_with("[offset") {
                lxlrc_lines.push(line.to_string());
                lrc_lines.push(line.to_string());
            }
            if line_time2.is_match(line) {
                lrc_lines.push(line.to_string());
            }
            continue;
        };
        let start_ms: i64 = caps[1].parse().unwrap_or(0);
        let start_time_str = ms_format(start_ms);
        if start_time_str.is_empty() {
            continue;
        }

        let words = line_time.replace(line, "");
        lrc_lines.push(format!(
            "{}{}",
            start_time_str,
            word_time.replace_all(&words, "")
        ));

        if !word_time.is_match(&words) {
            continue;
        }
        let times: Vec<String> = word_time
            .find_iter(&words)
            .map(|m| {
                let wc = regex::Regex::new(r"\((\d+),(\d+)\)")
                    .unwrap()
                    .captures(m.as_str())
                    .expect("word_time match implies captures");
                let w_start: i64 = wc[1].parse().unwrap_or(0);
                format!("<{},{}>", (w_start - start_ms).max(0), &wc[2])
            })
            .collect();
        let word_arr: Vec<&str> = word_time.split(&words).collect();
        // 上游 JS 在词数少于时间数时会拼出 "undefined"，此处按空串处理（避免污染歌词）
        let new_words: String = times
            .iter()
            .enumerate()
            .map(|(index, t)| format!("{}{}", t, word_arr.get(index).copied().unwrap_or("")))
            .collect();
        lxlrc_lines.push(format!("{}{}", start_time_str, new_words));
    }

    (lrc_lines.join("\n"), lxlrc_lines.join("\n"))
}

fn parse_rlyric(lrc: &str) -> String {
    let lrc = lrc.trim().replace('\r', "");
    if lrc.is_empty() {
        return String::new();
    }
    let line_time = line_time_re();
    let word_time = word_time_re();
    let mut lrc_lines: Vec<String> = Vec::new();

    for line in lrc.split('\n') {
        let line = line.trim();
        let Some(caps) = line_time.captures(line) else {
            continue;
        };
        let start_ms: i64 = caps[1].parse().unwrap_or(0);
        let start_time_str = ms_format(start_ms);
        if start_time_str.is_empty() {
            continue;
        }
        let words = line_time.replace(line, "");
        lrc_lines.push(format!(
            "{}{}",
            start_time_str,
            word_time.replace_all(&words, "")
        ));
    }
    lrc_lines.join("\n")
}

/// 逐行对齐时间标签；`pad_frac` 为 true 时先把小数补到 3 位再比较（tlrc 用）
fn fix_time_tag(target: &str, lrc: &str, pad_frac: bool) -> String {
    let line_time2 = line_time2_re();
    let mut lrc_lines: Vec<&str> = lrc.split('\n').collect();
    let mut new_lrc: Vec<String> = Vec::new();

    for line in target.split('\n') {
        let Some(caps) = line_time2.captures(line) else {
            continue;
        };
        let words = line_time2.replace(line, "");
        if words.trim().is_empty() {
            continue;
        }
        let mut time = caps[1].to_string();
        if pad_frac {
            if let Some(dot) = time.find('.') {
                // 上游 time.split('.')[1] 只取首个点后的段
                let first_seg_len = time[dot + 1..].split('.').next().unwrap_or("").len();
                if first_seg_len < 3 {
                    time.push_str(&"0".repeat(3 - first_seg_len));
                }
            }
        }
        let t1 = get_intv(&time);

        while !lrc_lines.is_empty() {
            let lrc_line = lrc_lines.remove(0);
            let Some(lrc_caps) = line_time2.captures(lrc_line) else {
                continue;
            };
            let t2 = get_intv(&lrc_caps[1]);
            if (t1 - t2).abs() < 100 {
                new_lrc.push(
                    line_time2
                        .replacen(line, 1, &lrc_caps[0])
                        .to_string(),
                );
                break;
            }
        }
    }
    new_lrc.join("\n")
}

/// 编排解析（上游 parseTools.parse）：
/// lrc -> parseLyric(removeTag)；rlrc -> fixRlrcTimeTag(parseRlyric(removeTag))；
/// tlrc -> fixTlrcTimeTag（注意 tlrc 不经 removeTag/parseRlyric）
pub fn parse(lrc: &str, tlrc: &str, rlrc: &str) -> LyricInfo {
    let mut lyric = String::new();
    let mut lxlyric = String::new();
    let mut rlyric = String::new();
    let mut tlyric = String::new();

    if !lrc.is_empty() {
        let (l, lx) = parse_lyric(&remove_tag(lrc));
        lyric = l;
        lxlyric = lx;
    }
    if !rlrc.is_empty() {
        rlyric = fix_time_tag(&parse_rlyric(&remove_tag(rlrc)), &lyric, false);
    }
    if !tlrc.is_empty() {
        tlyric = fix_time_tag(tlrc, &lyric, true);
    }

    LyricInfo {
        lyric,
        tlyric: if tlyric.is_empty() { None } else { Some(tlyric) },
        rlyric: if rlyric.is_empty() { None } else { Some(rlyric) },
        lxlyric: if lxlyric.is_empty() {
            None
        } else {
            Some(lxlyric)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/tx_qrc.json");

    #[test]
    fn test_decode_qrc_fixture() {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Fixture {
            lyric_hex: String,
            trans_hex: String,
            roma_hex: String,
            decoded: Decoded,
        }
        #[derive(serde::Deserialize)]
        struct Decoded {
            lyric: String,
            tlyric: String,
            rlyric: String,
        }
        let f: Fixture = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(decode_qrc(&f.lyric_hex).unwrap(), f.decoded.lyric);
        assert_eq!(decode_qrc(&f.trans_hex).unwrap(), f.decoded.tlyric);
        assert_eq!(decode_qrc(&f.roma_hex).unwrap(), f.decoded.rlyric);
    }

    #[test]
    fn test_parse_fixture() {
        #[derive(serde::Deserialize)]
        struct Fixture {
            decoded: Decoded,
            parsed: Parsed,
        }
        #[derive(serde::Deserialize)]
        struct Decoded {
            lyric: String,
            tlyric: String,
            rlyric: String,
        }
        #[derive(serde::Deserialize)]
        struct Parsed {
            lyric: String,
            tlyric: String,
            rlyric: String,
            lxlyric: String,
        }
        let f: Fixture = serde_json::from_str(FIXTURE).unwrap();
        let info = parse(&f.decoded.lyric, &f.decoded.tlyric, &f.decoded.rlyric);
        assert_eq!(info.lyric, f.parsed.lyric);
        assert_eq!(info.tlyric.as_deref().unwrap_or(""), f.parsed.tlyric);
        assert_eq!(info.rlyric.as_deref().unwrap_or(""), f.parsed.rlyric);
        assert_eq!(info.lxlyric.as_deref().unwrap_or(""), f.parsed.lxlyric);
    }

    #[test]
    fn test_decode_qrc_invalid() {
        assert_eq!(decode_qrc("").unwrap(), "");
        assert_eq!(decode_qrc("abc").unwrap(), ""); // 奇数长度
        assert_eq!(decode_qrc("zzzz").unwrap(), ""); // 非 hex
    }

    #[test]
    fn test_ms_format() {
        assert_eq!(ms_format(0), "[00:00.000]");
        assert_eq!(ms_format(324_324), "[05:24.324]");
        assert_eq!(ms_format(7_256), "[00:07.256]");
        assert_eq!(ms_format(3_661_001), "[61:01.001]");
    }

    #[test]
    fn test_get_intv() {
        assert_eq!(get_intv("00:07.256"), 7256);
        // 上游 JS `const [m, s, ms] = arr` 只解构前 3 段，第 4 段被丢弃：
        // ['1','02','03','004'] -> 1*3600000 + 2*1000 + 3 = 3602003（行为等价，非语义修正）
        assert_eq!(get_intv("1:02:03.004"), 3_602_003);
        // JS: '5' -> '5.0'.split(/:|\./) -> ['5','0'] -> unshift -> ['0','5','0'] -> 0+5*1000+0
        assert_eq!(get_intv("5"), 5_000);
        assert_eq!(get_intv(""), 0);
    }

    #[test]
    fn test_remove_tag() {
        let raw = r#"<?xml version="1.0"?><QrcInfos><Lyric_1 LyricType="1" LyricContent="[00:01.00]hello"/><foo bar="x"/></QrcInfos>"#;
        assert_eq!(remove_tag(raw), "[00:01.00]hello");
    }
}
