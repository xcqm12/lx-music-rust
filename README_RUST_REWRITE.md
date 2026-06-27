# LX Music Mobile - Rust 核心重写版

基于原有 LX Music Mobile 项目，保留 React Native UI 层，用 Rust 通过 FFI (JNI) 替换核心逻辑层（播放引擎、音源解析、歌词处理）。

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                     React Native (TypeScript/TSX)            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐ │
│  │   UI     │  │  Player  │  │  Source  │  │    Lyric     │ │
│  │ Screens  │  │  Plugin   │  │  Plugin  │  │   Plugin     │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────┬───────┘ │
│       │             │             │               │         │
│       └─────────────┴─────────────┴───────────────┘         │
│                       │                                      │
│              TurboModule (React Native)                     │
│              LXMusicCoreModule.kt                           │
└───────────────────────┬─────────────────────────────────────┘
                        │
┌───────────────────────┼─────────────────────────────────────┐
│              Android (Kotlin/Java)                           │
│                   RustBridge.kt                              │
│              (JNI 静态方法声明)                                │
└───────────────────────┬─────────────────────────────────────┘
                        │  JNI
┌───────────────────────┼─────────────────────────────────────┐
│                  Rust Core (liblx_music_core.so)             │
│  ┌────────────────────┼───────────────────────────────────┐ │
│  │              player.rs                                 │ │
│  │  ┌─────────────────┼────────────────────────────────┐ │ │
│  │  │  Audio Engine   │  Playlist    │  State Manager  │ │ │
│  │  └─────────────────┴────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              music_source.rs                            │ │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌───────┐ │ │
│  │  │  Kw    │ │  Kg    │ │  Tx    │ │  Wy    │ │  Mg   │ │ │
│  │  │(酷我)  │ │(酷狗)  │ │(QQ)   │ │(网易)  │ │(咪咕) │ │ │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └───────┘ │ │
│  └────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              lyric.rs                                   │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────────────┐│ │
│  │  │  Parser  │ │   Sync   │ │        Manager           ││ │
│  │  │(LRC/KRC) │ │(Timing)  │ │    (Cache/Events)        ││ │
│  │  └──────────┘ └──────────┘ └──────────────────────────┘│ │
│  └────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              Shared Utilities                           │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐ │ │
│  │  │http_utils│ │crypto_utils│ │js_engine │ │jni_bridge │ │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └───────────┘ │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 项目结构

```
lx-music-mobile-master/
├── src/
│   ├── rust/                          # Rust 核心库
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                 # 库入口
│   │       ├── player.rs              # 播放引擎
│   │       ├── music_source.rs        # 音乐源解析
│   │       ├── lyric.rs               # 歌词处理
│   │       ├── js_engine.rs           # JS 脚本引擎 (rquickjs)
│   │       ├── http_utils.rs          # HTTP 工具
│   │       ├── crypto_utils.rs        # 加密/解密工具
│   │       └── jni_bridge.rs          # JNI 桥接层
│   ├── android/                       # Android 原生桥接
│   │   ├── RustBridge.kt              # JNI 接口声明
│   │   ├── LXMusicCoreModule.kt       # TurboModule 定义
│   │   └── LXMusicPackage.kt          # RN Package 注册
│   ├── cpp/                           # C++ TurboModule 桥接 (可选)
│   │   ├── include/
│   │   │   └── LXMusicModule.hpp
│   │   └── LXMusicModule.cpp
│   └── ...                            # 原有 React Native 代码
├── android/
│   └── app/
│       ├── build.gradle
│       └── src/main/
│           ├── cpp/
│           │   ├── CMakeLists.txt
│           │   └── jni/
│           │       └── lx_music_jni.cpp
│           └── java/cn/toside/music/mobile/
│               ├── MainApplication.java
│               ├── RustBridgeKotlin.kt    # JNI 桥接实现
│               └── LXMusicCoreModule.kt   # TurboModule
└── package.json
```

## 技术栈

| 层级 | 技术 |
|------|------|
| UI | React Native 0.73 + TypeScript |
| 桥接 | TurboModule (RN 0.68+) |
| JNI | Kotlin → Rust FFI |
| 核心 | Rust (rquickjs, serde, reqwest, aes, hmac) |
| 构建 | Gradle 8.8 + CMake 3.18.1 + NDK 26 |

## Rust 依赖

```toml
[dependencies]
aes = "0.8"
cbc = "0.1"
hmac = "0.12"
rand = "0.8"
sha2 = "0.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["blocking", "json"] }
regex = "1.10"
crossbeam-channel = "0.5"
rquickjs = { version = "0.6", features = ["bindgen", "full-async"], optional = true }

[features]
default = []           # 默认禁用 JS 引擎 (Windows 编译)
js-engine = ["dep:rquickjs"]
```

## 构建指南

### 环境准备

- **Node.js** >= 18, **npm** >= 8.5.2
- **Rust** (stable) + Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`)
- **Android Studio** + SDK 34 + NDK 26+
- **CMake** 3.18+

### 构建步骤

**Windows (PowerShell):**

```powershell
# 1. 安装依赖
npm install

# 2. 设置 NDK 环境变量
$env:ANDROID_NDK_HOME = "$env:LOCALAPPDATA\Android\Sdk\ndk\<version>"

# 3. 构建 Rust 核心 (.so)
cd src/rust
cargo ndk -t arm64-v8a -o "..\..\android\app\src\main\jniLibs" build --release

# 4. 构建 Android APK
cd ..\..\android
.\gradlew assembleDebug --no-daemon
```

**注意：** `android/app/build.gradle` 中 `jniLibs.srcDirs` 必须指向 `src/main/jniLibs`，不能指向 `src/rust/target`（后者包含 `aarch64-linux-android/release/` 等非标准 ABI 子目录，会导致 `mergeDebugNativeLibs` 失败）。

### 调试运行

```bash
# 启动 Metro 开发服务器
npm start

# 安装并运行到设备
npx react-native run-android --active-arch-only

# 打开调试菜单
adb shell input keyevent 82
```

## 当前进度

### 已完成

- [x] 项目结构搭建
- [x] Rust 核心库框架 (player, music_source, lyric)
- [x] 加密/解密工具 (AES, HMAC, SHA2)
- [x] HTTP 工具 (reqwest)
- [x] JS 脚本引擎 (rquickjs, 可选)
- [x] JNI 桥接层 (RustBridge)
- [x] TurboModule 集成 (LXMusicCoreModule)
- [x] Android Gradle 构建配置
- [x] CMake + JNI 编译配置
- [x] Gradle 构建成功 (assembleDebug)
- [x] 设备安装运行成功
- [x] 清理 Jetpack Compose 残留文件
- [x] Rust .so 编译并集成到 APK (`liblx_music_core.so` 1.7MB, 已打包到 arm64-v8a 等架构 APK)
- [x] 完整 JNI 方法实现 (30 个方法: 初始化 2, 音源 6, 播放 15, 歌词 7)
- [x] 音乐源 Rust 原生实现 (kw/酷我, kg/酷狗, mg/咪咕; tx/QQ, wy/网易保留 JS 引擎)
- [x] 音频引擎集成 (Symphonia 解码 + AudioOutput 输出)
- [x] 单元测试 (128 个测试全部通过)

### 待完成

- [x] 端到端测试 (19 个集成测试，覆盖全链路: 初始化/播放/歌词/解码/音源/加密/并发)
- [x] 性能优化 (Regex 缓存、VecDeque 缓冲队列、二分搜索歌词查找、played_list 限长、Vec::with_capacity)
- [x] 正式版打包 (Release APK: arm64-v8a, armeabi-v7a, x86, x86_64, universal)

## 构建记录

| 日期 | 类型 | 结果 | 说明 |
|------|------|------|------|
| 2026-06-27 | Debug | 成功 | 5 架构 APK 全部生成 |
| 2026-06-27 | Release | 成功 | 5 架构 APK 全部生成，R8 代码混淆已启用 |

## 改进点

### JNI API 参考

`jni_bridge.rs` 实现了完整的 JNI 桥接层，共 30 个方法，与 [RustBridgeKotlin.kt](file:///c:/Users/Administrator/Desktop/lx-music-mobile-master/android/app/src/main/java/cn/toside/music/mobile/RustBridgeKotlin.kt) 一一对应：

| 分类 | 方法 | 返回 | 说明 |
|------|------|------|------|
| 初始化 | `initEngine` | Boolean | 初始化所有引擎 (Player, Lyric, JS) |
| 初始化 | `isInitialized` | Boolean | 检查引擎是否已初始化 |
| 音源 | `loadSource` | String | 加载音乐源 JS 脚本 |
| 音源 | `searchMusic` | String (JSON) | 搜索音乐 |
| 音源 | `getMusicUrl` | String (JSON) | 获取音乐播放 URL |
| 音源 | `getLyric` | String (JSON) | 获取歌词 |
| 音源 | `getPic` | String (JSON) | 获取专辑封面 |
| 音源 | `getSources` | String[] | 获取所有已加载的音源 |
| 播放 | `playerPlay` | void | 开始播放 |
| 播放 | `playerPause` | void | 暂停 |
| 播放 | `playerStop` | void | 停止 |
| 播放 | `playerToggle` | void | 切换播放/暂停 |
| 播放 | `playerNext` | void | 下一首 |
| 播放 | `playerPrev` | void | 上一首 |
| 播放 | `playerSeek` | void | 跳转到指定位置 |
| 播放 | `playerSetVolume` | void | 设置音量 (0.0-1.0) |
| 播放 | `playerSetPlaybackRate` | void | 设置播放速率 (0.5-2.0) |
| 播放 | `playerSetPlayMode` | void | 设置播放模式 (0-3) |
| 播放 | `playerSetPlaylist` | String | 设置播放列表 (JSON) |
| 播放 | `playerPlayAtIndex` | void | 播放指定索引 |
| 播放 | `playerGetState` | String (JSON) | 获取播放器状态 |
| 播放 | `playerAddToPlaylist` | String | 添加音乐到列表 |
| 播放 | `playerRemoveFromPlaylist` | void | 从列表移除 |
| 播放 | `playerClearPlaylist` | void | 清空播放列表 |
| 歌词 | `lyricSetLyric` | void | 设置歌词 (LRC + 翻译) |
| 歌词 | `lyricGetCurrentLine` | String (JSON) | 获取当前时间歌词行 |
| 歌词 | `lyricGetLineIndex` | Int | 获取当前歌词行索引 |
| 歌词 | `lyricGetLines` | String (JSON) | 获取所有歌词行 |
| 歌词 | `lyricSetPlaybackRate` | void | 设置歌词播放速率 |
| 歌词 | `lyricToggleTranslation` | void | 切换翻译显示 |
| 歌词 | `lyricIsShowTranslation` | Boolean | 是否显示翻译 |
| 歌词 | `lyricClear` | void | 清空歌词 |

### Rust 原生音源

`src/rust/src/sources/` 模块实现了纯 Rust 音乐源，无需 JS 引擎即可工作：

| 音源 | 文件 | 状态 | 说明 |
|------|------|------|------|
| 酷我 (kw) | `sources/kw.rs` | 原生 | 搜索、歌词、封面、URL 均通过 HTTP API |
| 酷狗 (kg) | `sources/kg.rs` | 原生 | 搜索、KRC 歌词解析、封面、URL |
| 咪咕 (mg) | `sources/mg.rs` | 原生 | 搜索、歌词、封面、URL |
| QQ (tx) | JS 引擎 | 保留 | 需要 `signRequest` 加密签名 |
| 网易 (wy) | JS 引擎 | 保留 | 需要 `eapi/weapi` 加密 |

**架构设计：**
- `MusicSourceApi` trait：定义统一的音源接口 (`search`, `getMusicUrl`, `getLyric`, `getPic`)
- `SourceManager`：管理原生和 JS 音源，自动路由
- 初始化时自动注册 3 个原生音源 + 2 个 JS 音源

### 性能提升
- **Rust 核心**：零成本抽象，无 GC 停顿
- **网络请求**：Rust 异步运行时 (reqwest)
- **歌词解析**：Rust 正则在原生层执行，无 JS 桥接开销

### Symphonia 音频引擎

`src/rust/src/audio_decoder.rs` 和 `audio_output.rs` 实现纯 Rust 音频处理：

| 模块 | 功能 | 说明 |
|------|------|------|
| `audio_decoder.rs` | 音频解码 | Symphonia 解码 MP3/AAC/FLAC/WAV → PCM |
| `audio_output.rs` | 音频输出 | PCM 缓冲队列 + Android AudioTrack 接口 |

**JNI 接口 (6 个新方法)：**
| 方法 | 功能 |
|------|------|
| `audioProbeFormat` | 探测音频格式 (codec/sampleRate/channels/duration) |
| `audioDecode` | 解码为 PCM 并返回 base64 |
| `audioDecodeResampled` | 解码 + 重采样到目标采样率 |
| `audioQueueBuffer` | 队列 PCM 数据到播放缓冲区 |
| `audioDequeueBuffer` | 取出缓冲区 PCM 供 AudioTrack 消费 |
| `audioBufferSizeBytes` | 获取推荐缓冲区大小 |

**解码流程：**
```
HTTP 音频 URL → 下载字节 → Symphonia 探测格式 → 解码为 i16 PCM
→ 重采样(可选) → 缓冲队列 → JNI 取出 → Android AudioTrack 播放
```

### 代码质量
- **类型安全**：Rust 强类型系统避免运行时错误
- **内存安全**：Rust 所有权系统消除内存泄漏和悬垂指针
- **模块化**：清晰的模块划分

### 架构优势
- **渐进式迁移**：保留 RN UI，逐步替换底层逻辑
- **低风险**：UI 层不变，用户无感知
- **TurboModule**：使用 RN 官方推荐的 JSI 桥接，性能优于传统 Bridge