//! End-to-End Integration Tests
//!
//! Tests the full Rust core pipeline from initialization through
//! music source management, playback control, lyric processing,
//! and audio decoding — all without requiring Android/device.

use lx_music_core::*;

// ============================================================================
// Test helpers
// ============================================================================

fn make_music_item(id: &str, name: &str, singer: &str) -> MusicItem {
    MusicItem {
        id: id.to_string(),
        name: name.to_string(),
        singer: singer.to_string(),
        source: "kw".to_string(),
        album_id: None,
        album_name: None,
        duration: None,
        pic_url: None,
        lrc_url: None,
        url: None,
    }
}

fn make_music_info(id: &str, name: &str, singer: &str) -> MusicInfo {
    MusicInfo {
        id: id.to_string(),
        name: name.to_string(),
        singer: singer.to_string(),
        source: "kw".to_string(),
        album_id: None,
        album_name: None,
        duration: None,
        pic_url: None,
        lrc_url: None,
        qualitys: vec![],
        url: None,
    }
}

fn create_minimal_wav(sample_rate: u32, channels: u16, num_samples: u32) -> Vec<u8> {
    let data_size = num_samples * channels as u32 * 2; // 16-bit
    let file_size = 36 + data_size;
    let mut wav = Vec::with_capacity(44 + data_size as usize);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * 2;
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&(channels * 2).to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.resize(44 + data_size as usize, 0u8);

    wav
}

fn sample_lyric() -> &'static str {
    "[00:01.000]第一行歌词\n[00:04.000]第二行歌词\n[00:07.500]第三行歌词\n[00:10.000]第四行歌词\n[00:13.000]第五行歌词\n"
}

fn sample_translation() -> &'static str {
    "[00:01.000]Line one\n[00:04.000]Line two\n[00:07.500]Line three\n[00:10.000]Line four\n[00:13.000]Line five\n"
}

// ============================================================================
// Test 1: Full initialization lifecycle
// ============================================================================

#[test]
fn e2e_initialization_lifecycle() {
    // 1. Create engines
    let player = PlayerEngine::new();
    let lyric = LyricEngine::new();

    // 2. Verify initial state
    let state = player.get_state();
    assert!(!state.is_playing);
    assert!(!state.is_paused);
    assert!(state.playlist.is_empty());
    assert_eq!(state.play_mode, PlayMode::ListLoop);
    assert_eq!(state.volume, 0.8);
    assert_eq!(state.playback_rate, 1.0);

    // 3. Lyric engine default state
    let lines = lyric.get_lines();
    assert!(lines.is_empty());
    assert!(!lyric.is_show_translation());

    // 4. Source manager initialized
    let mgr = get_source_manager();
    {
        let mut mgr = mgr.write().unwrap();
        mgr.register_native(Box::new(sources::kw::KwSource::new()));
        mgr.register_native(Box::new(sources::kg::KgSource::new()));
        mgr.register_native(Box::new(sources::mg::MgSource::new()));
        mgr.register_js("tx", "QQ音乐");
        mgr.register_js("wy", "网易音乐");
    }

    let sources = mgr.read().unwrap().get_source_list();
    assert!(sources.len() >= 5);
    assert!(sources.iter().any(|s| s.id == "kw" && s.is_native));
    assert!(sources.iter().any(|s| s.id == "tx" && !s.is_native));
}

// ============================================================================
// Test 2: Full playback lifecycle
// ============================================================================

#[test]
fn e2e_playback_lifecycle() {
    let player = PlayerEngine::new();

    // 1. Empty playlist → play should be no-op
    player.play();
    assert!(!player.get_state().is_playing);

    // 2. Set playlist with 5 songs
    let playlist = vec![
        make_music_item("1", "歌曲A", "歌手A"),
        make_music_item("2", "歌曲B", "歌手B"),
        make_music_item("3", "歌曲C", "歌手C"),
        make_music_item("4", "歌曲D", "歌手D"),
        make_music_item("5", "歌曲E", "歌手E"),
    ];
    player.set_playlist(playlist);
    assert_eq!(player.get_state().playlist.len(), 5);
    assert_eq!(player.get_state().current_index, 0);

    // 3. Play
    player.play();
    assert!(player.get_state().is_playing);
    assert!(!player.get_state().is_paused);

    // 4. Pause
    player.pause();
    assert!(!player.get_state().is_playing);
    assert!(player.get_state().is_paused);

    // 5. Resume
    player.play();
    assert!(player.get_state().is_playing);

    // 6. Next track (ListLoop mode)
    player.next();
    assert_eq!(player.get_state().current_index, 1);

    // 7. Prev track
    player.prev();
    assert_eq!(player.get_state().current_index, 0);

    // 8. Play at specific index
    player.play_at_index(3);
    assert_eq!(player.get_state().current_index, 3);
    assert!(player.get_state().is_playing);

    // 9. Stop
    player.stop();
    assert!(!player.get_state().is_playing);
    assert!(!player.get_state().is_paused);
    assert_eq!(player.get_state().progress.current_time, 0);

    // 10. Clear playlist
    player.clear_playlist();
    assert!(player.get_state().playlist.is_empty());
    assert!(player.get_state().current_music.is_none());
}

// ============================================================================
// Test 3: Play mode transitions
// ============================================================================

#[test]
fn e2e_play_mode_transitions() {
    let player = PlayerEngine::new();
    let playlist = vec![
        make_music_item("1", "A", "a"),
        make_music_item("2", "B", "b"),
        make_music_item("3", "C", "c"),
    ];
    player.set_playlist(playlist);

    // ListLoop: wraps around
    player.set_play_mode(PlayMode::ListLoop);
    player.next();
    assert_eq!(player.get_state().current_index, 1);
    player.next();
    assert_eq!(player.get_state().current_index, 2);
    player.next(); // wrap
    assert_eq!(player.get_state().current_index, 0);

    // List: stops at end
    player.set_play_mode(PlayMode::List);
    player.next();
    assert_eq!(player.get_state().current_index, 1);
    player.next();
    assert_eq!(player.get_state().current_index, 2);
    player.next(); // stay at end
    assert_eq!(player.get_state().current_index, 2);

    // SingleLoop: stays on same track
    player.set_play_mode(PlayMode::SingleLoop);
    player.next();
    assert_eq!(player.get_state().current_index, 2);
    player.next();
    assert_eq!(player.get_state().current_index, 2);

    // Random: result is within bounds
    player.set_play_mode(PlayMode::Random);
    player.next();
    let idx = player.get_state().current_index;
    assert!(idx < 3);
}

// ============================================================================
// Test 4: Full lyric pipeline
// ============================================================================

#[test]
fn e2e_lyric_pipeline() {
    let lyric = LyricEngine::new();

    // 1. Parse raw LRC
    let lines = lyric.parse_lrc(sample_lyric());
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0].time, 1.0);
    assert_eq!(lines[0].text, "第一行歌词");
    assert_eq!(lines[4].time, 13.0);

    // 2. Parse LRC file with translation
    let result = lyric.parse_lrc_file(sample_lyric());
    assert_eq!(result.lines.len(), 5);
    assert_eq!(result.raw_lyric.lines().count(), 5);

    // 3. Set lyric with translation
    lyric.set_lyric(sample_lyric(), sample_translation());
    let lines = lyric.get_lines();
    assert_eq!(lines.len(), 5);
    // Translations should be merged
    let has_translation = lines.iter().any(|l| l.translation.is_some());
    assert!(has_translation, "Translations should be merged");

    // 4. Get current line at various times
    let line = lyric.get_current_line(1500); // 1.5s
    assert!(line.is_some());
    assert_eq!(line.unwrap().text, "第一行歌词");

    let line = lyric.get_current_line(5000); // 5s
    assert!(line.is_some());
    assert_eq!(line.unwrap().text, "第二行歌词");

    // 5. Get line index
    assert_eq!(lyric.get_line_index(1500), 0);
    assert_eq!(lyric.get_line_index(5000), 1);
    assert_eq!(lyric.get_line_index(0), -1);

    // 6. Get lines with range
    let range = lyric.get_lines_with_range(2, 3);
    assert_eq!(range.len(), 3);

    // 7. JSON output
    let json = lyric.get_lines_json();
    assert!(json.contains("第一行歌词"));

    let line_json = lyric.get_current_line_json(1500);
    assert!(line_json.contains("第一行歌词"));

    // 8. Toggle translation
    lyric.toggle_translation(true);
    assert!(lyric.is_show_translation());
    lyric.toggle_translation(false);
    assert!(!lyric.is_show_translation());

    // 9. Clear
    lyric.clear();
    assert!(lyric.get_lines().is_empty());
}

// ============================================================================
// Test 5: Audio decoder pipeline
// ============================================================================

#[test]
fn e2e_audio_decoder_pipeline() {
    // 1. Create a minimal WAV file
    let wav_data = create_minimal_wav(44100, 2, 4410); // 0.1s stereo 44100Hz

    // 2. Probe format
    let format = AudioDecoder::probe_format(&wav_data);
    assert!(format.is_ok(), "Probe failed: {:?}", format.err());
    let fmt = format.unwrap();
    assert_eq!(fmt.sample_rate, 44100);
    assert_eq!(fmt.channels, 2);
    assert!(!fmt.codec.is_empty());

    // 3. Decode to PCM
    let decoded = AudioDecoder::decode(&wav_data);
    assert!(decoded.is_ok(), "Decode failed: {:?}", decoded.err());
    let audio = decoded.unwrap();
    assert_eq!(audio.sample_rate, 44100);
    assert_eq!(audio.channels, 2);
    assert!(!audio.samples.is_empty());
    // Should have 4410 samples * 2 channels = 8820 samples
    assert_eq!(audio.samples.len(), 4410 * 2);

    // 4. Verify PCM data is valid (silence → all zeros)
    for sample in &audio.samples {
        assert_eq!(*sample, 0i16);
    }
}

#[test]
fn e2e_audio_decoder_stereo_48000() {
    let wav_data = create_minimal_wav(48000, 2, 4800);
    let format = AudioDecoder::probe_format(&wav_data).unwrap();
    assert_eq!(format.sample_rate, 48000);
    assert_eq!(format.channels, 2);

    let decoded = AudioDecoder::decode(&wav_data).unwrap();
    assert_eq!(decoded.sample_rate, 48000);
    assert_eq!(decoded.channels, 2);
    assert_eq!(decoded.samples.len(), 4800 * 2);
}

#[test]
fn e2e_audio_decoder_mono() {
    let wav_data = create_minimal_wav(22050, 1, 2205);
    let format = AudioDecoder::probe_format(&wav_data).unwrap();
    assert_eq!(format.sample_rate, 22050);
    assert_eq!(format.channels, 1);

    let decoded = AudioDecoder::decode(&wav_data).unwrap();
    assert_eq!(decoded.sample_rate, 22050);
    assert_eq!(decoded.channels, 1);
    assert_eq!(decoded.samples.len(), 2205);
}

// ============================================================================
// Test 6: Audio output + player integration
// ============================================================================

#[test]
fn e2e_audio_output_player_integration() {
    let player = PlayerEngine::new();

    // 1. Initial audio output state should be idle
    let state = player.get_audio_output_state();
    assert_eq!(state, OutputState::Idle);

    // 2. Buffer size should be valid
    assert!(player.audio_buffer_size_bytes() > 0);

    // 3. Set playlist and play
    player.set_playlist(vec![make_music_item("1", "song", "artist")]);
    player.play();
    assert!(player.is_audio_playing());

    // 4. Queue PCM buffer and verify
    let samples = vec![100i16, 200, 300, 400, 500, 600];
    player.queue_audio_buffer(samples.clone(), 44100, 2);
    assert_eq!(player.audio_buffer_count(), 1);

    // 5. Dequeue and verify
    let buf = player.dequeue_audio_buffer().unwrap();
    assert_eq!(buf.samples, samples);
    assert_eq!(buf.sample_rate, 44100);
    assert_eq!(buf.channels, 2);
    assert_eq!(player.audio_buffer_count(), 0);

    // 6. Multiple buffers
    player.queue_audio_buffer(vec![1i16, 2], 44100, 1);
    player.queue_audio_buffer(vec![3i16, 4], 44100, 1);
    assert_eq!(player.audio_buffer_count(), 2);
    player.dequeue_audio_buffer();
    player.dequeue_audio_buffer();
    assert_eq!(player.audio_buffer_count(), 0);

    // 7. Stop resets audio
    player.stop();
    assert!(!player.is_audio_playing());
}

#[test]
fn e2e_audio_volume_and_rate() {
    let player = PlayerEngine::new();

    // Volume control
    player.set_volume(0.0);
    assert_eq!(player.get_state().volume, 0.0);
    player.set_volume(0.75);
    assert_eq!(player.get_state().volume, 0.75);
    player.set_volume(1.0);
    assert_eq!(player.get_state().volume, 1.0);

    // Clamping
    player.set_volume(2.0);
    assert_eq!(player.get_state().volume, 1.0);
    player.set_volume(-0.5);
    assert_eq!(player.get_state().volume, 0.0);

    // Playback rate
    player.set_playback_rate(1.0);
    assert_eq!(player.get_state().playback_rate, 1.0);
    player.set_playback_rate(1.5);
    assert_eq!(player.get_state().playback_rate, 1.5);
    player.set_playback_rate(3.0);
    assert_eq!(player.get_state().playback_rate, 2.0);
    player.set_playback_rate(0.25);
    assert_eq!(player.get_state().playback_rate, 0.5);
}

// ============================================================================
// Test 7: Full audio pipeline (decode → queue → dequeue)
// ============================================================================

#[test]
fn e2e_full_audio_pipeline() {
    let player = PlayerEngine::new();

    // 1. Decode a WAV file
    let wav_data = create_minimal_wav(44100, 2, 4410);
    let decoded = AudioDecoder::decode(&wav_data).unwrap();

    // 2. Set playlist and start playing
    player.set_playlist(vec![make_music_item("1", "song", "artist")]);
    player.play();

    // 3. Queue decoded PCM data
    player.queue_audio_buffer(decoded.samples, decoded.sample_rate, decoded.channels);
    assert_eq!(player.audio_buffer_count(), 1);

    // 4. Verify buffer contents
    let buf = player.dequeue_audio_buffer().unwrap();
    assert_eq!(buf.sample_rate, 44100);
    assert_eq!(buf.channels, 2);
    assert_eq!(buf.samples.len(), 4410 * 2);

    // 5. Stop
    player.stop();
    assert!(!player.is_audio_playing());
}

// ============================================================================
// Test 8: Crypto utilities in music pipeline context
// ============================================================================

#[test]
fn e2e_crypto_music_pipeline() {
    // Simulate typical music API signing flow
    let timestamp = "1234567890";
    let secret = "my_secret_key";

    // 1. Hash the request
    let hash = CryptoUtils::sha256(&format!("{}{}", timestamp, secret));
    assert_eq!(hash.len(), 64);

    // 2. Create signature
    let mut params = std::collections::HashMap::new();
    params.insert("timestamp".to_string(), timestamp.to_string());
    params.insert("method".to_string(), "search".to_string());
    params.insert("keyword".to_string(), "测试".to_string());
    let sig = CryptoUtils::create_signature(&params, secret);
    assert_eq!(sig.len(), 64);

    // 3. HMAC for authentication
    let hmac = CryptoUtils::hmac_sha256(secret.as_bytes(), timestamp.as_bytes());
    assert_eq!(hmac.len(), 32);

    // 4. Base64 encode for URL parameter
    let encoded = CryptoUtils::base64_encode(&sig);
    let decoded = CryptoUtils::base64_decode(&encoded).unwrap();
    assert_eq!(decoded, sig);

    // 5. Random nonce generation
    let nonce = CryptoUtils::random_hex(16);
    assert_eq!(nonce.len(), 32);

    // 6. AES encryption (for sensitive data in transit)
    let key = b"12345678901234567890123456789012";
    let iv = b"1234567890123456";
    let encrypted = CryptoUtils::aes_encrypt("secret_data", key, iv).unwrap();
    let decrypted = CryptoUtils::aes_decrypt(&encrypted, key, iv).unwrap();
    assert_eq!(decrypted, "secret_data");
}

// ============================================================================
// Test 9: Source manager integration
// ============================================================================

#[test]
fn e2e_source_manager_integration() {
    let mgr = get_source_manager();

    // 1. Register all sources
    {
        let mut mgr = mgr.write().unwrap();
        mgr.register_native(Box::new(sources::kw::KwSource::new()));
        mgr.register_native(Box::new(sources::kg::KgSource::new()));
        mgr.register_native(Box::new(sources::mg::MgSource::new()));
        mgr.register_js("tx", "QQ音乐");
        mgr.register_js("wy", "网易音乐");
    }

    // 2. Verify source list
    {
        let mgr = mgr.read().unwrap();
        let list = mgr.get_source_list();
        assert!(list.len() >= 5, "Expected at least 5 sources, got {}", list.len());

        // 3. Check native vs JS
        assert!(mgr.is_native("kw"));
        assert!(mgr.is_native("kg"));
        assert!(mgr.is_native("mg"));
        assert!(!mgr.is_native("tx"));
        assert!(!mgr.is_native("wy"));
        assert!(!mgr.is_native("nonexistent"));

        // 4. Verify source info
        let kw_entry = list.iter().find(|s| s.id == "kw").unwrap();
        assert!(kw_entry.is_native);
        assert_eq!(kw_entry.name, "酷我音乐");

        let tx_entry = list.iter().find(|s| s.id == "tx").unwrap();
        assert!(!tx_entry.is_native);
        assert_eq!(tx_entry.name, "QQ音乐");
    }
}

// ============================================================================
// Test 10: Player + Lyric coordinated pipeline
// ============================================================================

#[test]
fn e2e_player_lyric_coordination() {
    let player = PlayerEngine::new();
    let lyric = LyricEngine::new();

    // 1. Set up playlist
    player.set_playlist(vec![
        make_music_item("1", "song", "artist"),
        make_music_item("2", "song2", "artist2"),
    ]);

    // 2. Set up lyrics
    lyric.set_lyric(sample_lyric(), sample_translation());

    // 3. Start playing
    player.play();
    assert!(player.get_state().is_playing);

    // 4. Simulate progress updates with lyric sync
    let progress_points = vec![
        (1500, 0, "第一行歌词"),   // 1.5s → line 0
        (5000, 1, "第二行歌词"),   // 5.0s → line 1
        (9000, 2, "第三行歌词"),   // 9.0s → line 2
        (12000, 3, "第四行歌词"),  // 12.0s → line 3
    ];

    for (time_ms, expected_idx, expected_text) in &progress_points {
        player.update_progress(*time_ms, 30000);
        let line = lyric.get_current_line(*time_ms).unwrap();
        assert_eq!(line.text, *expected_text);
        assert_eq!(lyric.get_line_index(*time_ms), *expected_idx as i32);
    }

    // 5. Seek changes lyric position
    player.seek(8000);
    let line = lyric.get_current_line(8000).unwrap();
    assert_eq!(line.text, "第三行歌词");

    // 6. Next track
    player.next();
    assert_eq!(player.get_state().current_index, 1);

    // 7. Stop
    player.stop();
    assert!(!player.get_state().is_playing);
}

// ============================================================================
// Test 11: Playlist manipulation + state consistency
// ============================================================================

#[test]
fn e2e_playlist_state_consistency() {
    let player = PlayerEngine::new();

    // 1. Add items one by one
    player.add_to_playlist(make_music_item("1", "A", "a"));
    player.add_to_playlist(make_music_item("2", "B", "b"));
    player.add_to_playlist(make_music_item("3", "C", "c"));
    assert_eq!(player.get_state().playlist.len(), 3);

    // 2. Remove middle item
    player.remove_from_playlist(1);
    let state = player.get_state();
    assert_eq!(state.playlist.len(), 2);
    assert_eq!(state.playlist[0].id, "1");
    assert_eq!(state.playlist[1].id, "3");

    // 3. Remove from JSON
    player.add_to_playlist_json(r#"{"id":"4","name":"D","singer":"d","source":"kw"}"#).unwrap();
    assert_eq!(player.get_state().playlist.len(), 3);

    // 4. Set playlist from JSON
    let json = r#"[{"id":"10","name":"X","singer":"x","source":"kw"},{"id":"20","name":"Y","singer":"y","source":"kw"}]"#;
    player.set_playlist_json(json).unwrap();
    assert_eq!(player.get_state().playlist.len(), 2);

    // 5. State JSON contains all required fields
    let state_json = player.get_state_json();
    assert!(state_json.contains("isPlaying"));
    assert!(state_json.contains("playlist"));
    assert!(state_json.contains("playMode"));
    assert!(state_json.contains("volume"));
    assert!(state_json.contains("playbackRate"));

    // 6. MusicItem ↔ MusicInfo conversion
    let info = MusicInfo {
        id: "conv".to_string(),
        name: "Test".to_string(),
        singer: "Tester".to_string(),
        source: "kw".to_string(),
        album_id: Some("alb1".to_string()),
        album_name: Some("Album".to_string()),
        duration: Some("03:30".to_string()),
        pic_url: Some("http://pic.url".to_string()),
        lrc_url: Some("http://lrc.url".to_string()),
        qualitys: vec![QualityInfo {
            quality: "320k".to_string(),
            size: Some("10MB".to_string()),
            url: Some("http://audio.url".to_string()),
        }],
        url: Some("http://audio.url".to_string()),
    };
    let item = MusicItem::from(info);
    assert_eq!(item.id, "conv");
    assert_eq!(item.album_id, Some("alb1".to_string()));
}

// ============================================================================
// Test 12: Lyric playback rate effects
// ============================================================================

#[test]
fn e2e_lyric_playback_rate() {
    let lyric = LyricEngine::new();
    lyric.set_raw_lyric("[00:01.000]A\n[00:02.000]B\n[00:03.000]C\n[00:04.000]D");

    // At 1.0x, 2500ms real = 2500ms lyric → line 1 (B)
    lyric.set_playback_rate(1.0);
    assert_eq!(lyric.get_line_index(2500), 1);
    assert_eq!(lyric.get_current_line(2500).unwrap().text, "B");

    // At 2.0x, 2500ms real = 1250ms lyric → line 0 (A)
    lyric.set_playback_rate(2.0);
    assert_eq!(lyric.get_line_index(2500), 0);
    assert_eq!(lyric.get_current_line(2500).unwrap().text, "A");

    // At 0.5x, 2500ms real = 5000ms lyric → line 4 (beyond last, returns D)
    lyric.set_playback_rate(0.5);
    let line = lyric.get_current_line(2500).unwrap();
    assert_eq!(line.text, "D");

    // Reset
    lyric.set_playback_rate(1.0);
    assert_eq!(lyric.get_line_index(1500), 0);
}

// ============================================================================
// Test 13: Edge cases and error handling
// ============================================================================

#[test]
fn e2e_edge_cases() {
    // 1. Empty playlist operations
    let player = PlayerEngine::new();
    player.play();
    assert!(!player.get_state().is_playing);
    player.next();
    assert!(player.get_state().current_music.is_none());
    player.prev();
    assert!(player.get_state().current_music.is_none());
    player.remove_from_playlist(99); // shouldn't panic
    player.play_at_index(99); // shouldn't panic

    // 2. Empty lyric operations
    let lyric = LyricEngine::new();
    assert!(lyric.get_current_line(0).is_none());
    assert_eq!(lyric.get_line_index(0), -1);
    assert_eq!(lyric.get_lines_json(), "[]");
    assert_eq!(lyric.get_current_line_json(0), "null");
    assert!(lyric.get_lines_with_range(0, 5).is_empty());
    assert_eq!(lyric.get_lyric_time(0), None);

    // 3. Large volume / rate values
    player.set_volume(100.0);
    assert_eq!(player.get_state().volume, 1.0);
    player.set_playback_rate(100.0);
    assert_eq!(player.get_state().playback_rate, 2.0);

    // 4. Invalid JSON parsing
    assert!(player.set_playlist_json("not json").is_err());
    assert!(player.add_to_playlist_json("not json").is_err());

    // 5. Audio operations on empty state
    assert!(player.dequeue_audio_buffer().is_none());
    assert_eq!(player.audio_buffer_count(), 0);
}

// ============================================================================
// Test 14: Serialization round-trip
// ============================================================================

#[test]
fn e2e_serialization_roundtrip() {
    // 1. PlayerState JSON round-trip
    let player = PlayerEngine::new();
    player.set_playlist(vec![
        make_music_item("1", "A", "a"),
        make_music_item("2", "B", "b"),
    ]);
    player.set_play_mode(PlayMode::Random);
    player.set_volume(0.5);
    player.set_playback_rate(1.25);
    player.play();

    let json = player.get_state_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["isPlaying"], true);
    assert_eq!(parsed["playlist"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["volume"], 0.5);
    assert_eq!(parsed["playbackRate"], 1.25);

    // 2. LyricData JSON round-trip
    let lyric = LyricEngine::new();
    lyric.set_lyric(sample_lyric(), sample_translation());
    let json = lyric.get_lines_json();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 5);

    // 3. MusicInfo JSON round-trip
    let info = make_music_info("123", "song", "artist");
    let json = serde_json::to_string(&info).unwrap();
    let parsed: MusicInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "123");
    assert_eq!(parsed.name, "song");
}

// ============================================================================
// Test 15: Concurrent access safety
// ============================================================================

#[test]
fn e2e_concurrent_player_access() {
    use std::sync::Arc;
    use std::thread;

    let player = Arc::new(PlayerEngine::new());
    player.set_playlist(vec![
        make_music_item("1", "A", "a"),
        make_music_item("2", "B", "b"),
    ]);
    player.play();

    let p1 = player.clone();
    let p2 = player.clone();
    let p3 = player.clone();

    let t1 = thread::spawn(move || {
        for _ in 0..10 {
            p1.set_volume(0.5);
            p1.set_playback_rate(1.0);
        }
    });

    let t2 = thread::spawn(move || {
        for _ in 0..10 {
            let _ = p2.get_state();
        }
    });

    let t3 = thread::spawn(move || {
        for _ in 0..10 {
            p3.update_progress(1000, 30000);
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    // State should still be consistent
    let state = player.get_state();
    assert!(state.playlist.len() > 0);
}

// ============================================================================
// Test 16: Stress test - large playlist
// ============================================================================

#[test]
fn e2e_stress_large_playlist() {
    let player = PlayerEngine::new();

    let playlist: Vec<MusicItem> = (0..1000)
        .map(|i| make_music_item(&i.to_string(), &format!("Song {}", i), &format!("Artist {}", i % 100)))
        .collect();

    player.set_playlist(playlist);
    assert_eq!(player.get_state().playlist.len(), 1000);

    // Navigate through playlist
    for _ in 0..50 {
        player.next();
    }
    assert_eq!(player.get_state().current_index, 50);

    for _ in 0..25 {
        player.prev();
    }
    assert_eq!(player.get_state().current_index, 25);

    // Play at specific index
    player.play_at_index(999);
    assert_eq!(player.get_state().current_index, 999);

    player.play_at_index(0);
    assert_eq!(player.get_state().current_index, 0);

    // Clear
    player.clear_playlist();
    assert!(player.get_state().playlist.is_empty());
}