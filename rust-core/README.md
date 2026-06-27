# LX Music Core - Rust 核心模块

本项目将 LX Music Mobile 的核心功能用 Rust 重写，包括播放引擎、音乐源解析和歌词处理。

## 项目结构

```
rust-core/
├── Cargo.toml          # 工作区配置
├── crates/
│   ├── common/         # 公共类型和工具
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── models.rs     # 数据模型（MusicInfo, LyricInfo 等）
│   │   │   ├── error.rs      # 错误类型
│   │   │   └── utils.rs      # 工具函数
│   │   └── Cargo.toml
│   │
│   ├── player/         # 播放引擎
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── audio_engine.rs   # 音频引擎（CPAL + Symphonia）
│   │   │   ├── decoder.rs        # 音频解码器
│   │   │   ├── playlist.rs       # 播放列表管理
│   │   │   └── events.rs         # 事件系统
│   │   └── Cargo.toml
│   │
│   ├── music-source/   # 音乐源解析
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── sources/          # 各音乐源实现
│   │   │   │   ├── kuwo.rs       # 酷我音乐
│   │   │   │   ├── kugou.rs      # 酷狗音乐
│   │   │   │   ├── qq_music.rs   # QQ音乐
│   │   │   │   ├── netease.rs    # 网易云音乐
│   │   │   │   └── migu.rs       # 咪咕音乐
│   │   │   ├── crypto.rs         # 加密/解密工具
│   │   │   └── utils.rs          # 通用工具
│   │   └── Cargo.toml
│   │
│   ├── lyric/          # 歌词处理
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── parser.rs         # LRC/KRC/YRC 解析
│   │   │   ├── sync.rs           # 歌词同步
│   │   │   ├── types.rs          # 歌词类型定义
│   │   │   └── manager.rs        # 歌词管理器
│   │   └── Cargo.toml
│   │
│   └── android/        # Android FFI 接口
│       ├── src/
│       │   ├── lib.rs
│       │   ├── player_ffi.rs     # 播放器 FFI
│       │   ├── music_source_ffi.rs # 音乐源 FFI
│       │   ├── lyric_ffi.rs      # 歌词 FFI
│       │   └── types.rs          # FFI 类型转换
│       └── Cargo.toml
```

## 功能特性

### 播放引擎
- 基于 CPAL 的跨平台音频播放
- 使用 Symphonia 进行音频解码（支持 MP3, FLAC, AAC 等）
- 网络音频流缓冲
- 播放列表管理（顺序、循环、随机、单曲循环）
- 播放进度追踪和事件通知

### 音乐源解析
支持以下音乐平台：
- 酷我音乐 (kw)
- 酷狗音乐 (kg)
- QQ音乐 (tx)
- 网易云音乐 (wy)
- 咪咕音乐 (mg)

功能：
- 歌曲搜索
- 获取音乐 URL（多音质支持）
- 歌词获取
- 跨源查找（当一个源失效时自动切换到其他源）

### 歌词处理
- LRC 格式解析
- KRC 格式解密（酷狗加密歌词）
- YRC 格式支持（网易云逐字歌词）
- QRC 格式支持（QQ音乐）
- 歌词同步和实时滚动
- 翻译和罗马音显示

## 构建

### 构建 Rust 库
```bash
cd rust-core
# Android 构建
# 需要先安装 Android NDK 和 cargo-ndk
cargo ndk -t armeabi-v7a -t arm64-v8a -t x86_64 -o ../android/app/src/main/jniLibs build --release
```

### 依赖
- Rust 1.70+
- Android NDK r25+
- cargo-ndk

## FFI 接口

Rust 核心通过 JNI 暴露以下接口给 Android：

### 播放器接口
- `initialize()` - 初始化播放器
- `play()`, `pause()`, `stop()` - 播放控制
- `seek(position)` - 跳转到指定位置
- `playTrack(musicInfo)` - 播放指定歌曲
- `next()`, `previous()` - 切歌

### 音乐源接口
- `initialize()` - 初始化音乐源管理器
- `search(source, keyword, page, limit)` - 搜索歌曲
- `getMusicUrl(musicInfo, quality)` - 获取音乐 URL
- `getLyric(musicInfo)` - 获取歌词

### 歌词接口
- `initialize()` - 初始化歌词管理器
- `loadLyric(lyricInfo)` - 加载歌词
- `getCurrentLine(timeMs)` - 获取当前行
- `getTranslation(timeMs)` - 获取翻译

## 许可证
MIT
