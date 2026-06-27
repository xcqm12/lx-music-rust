# LX Music Android - Jetpack Compose 版本

使用 Jetpack Compose 重写的 LX Music Android 客户端。

## 项目结构

```
android/
├── app/                    # 主应用模块
│   ├── src/main/java/com/lx/music/
│   │   ├── MainActivity.kt
│   │   ├── LXMusicApplication.kt
│   │   ├── navigation/     # 导航
│   │   │   └── LXMusicNavHost.kt
│   │   ├── ui/
│   │   │   ├── theme/      # 主题
│   │   │   ├── components/ # 通用组件
│   │   │   └── screens/    # 页面
│   │   │       ├── home/       # 首页
│   │   │       ├── search/     # 搜索
│   │   │       ├── player/     # 播放器
│   │   │       ├── playlist/   # 播放列表
│   │   │       └── settings/   # 设置
│   │   └── player/         # 播放器服务
│   └── build.gradle.kts
│
├── core/                   # 核心模块
│   └── src/main/java/com/lx/music/core/
│       ├── RustCoreBridge.kt   # Rust FFI 桥接
│       └── model/
│           └── Models.kt       # 数据模型
│
├── player/                 # 播放器模块
│   └── src/main/java/com/lx/music/player/
│       ├── PlayerBridge.kt     # 播放器 FFI
│       └── PlayerRepository.kt # 播放器存储库
│
├── music-source/           # 音乐源模块
│   └── src/main/java/com/lx/music/musicsource/
│       ├── MusicSourceBridge.kt
│       └── MusicSourceRepository.kt
│
└── lyric/                  # 歌词模块
    └── src/main/java/com/lx/music/lyric/
        ├── LyricBridge.kt
        └── LyricRepository.kt
```

## 技术栈

- **UI**: Jetpack Compose + Material3
- **架构**: MVVM + Repository 模式
- **依赖注入**: Hilt
- **导航**: Navigation Compose
- **图片加载**: Coil
- **异步**: Kotlin Coroutines + Flow
- **后端**: Rust (通过 JNI)

## 功能特性

### 首页
- 推荐歌曲
- 歌单
- 排行榜
- 底部迷你播放器

### 搜索
- 关键词搜索
- 搜索历史
- 热门搜索
- 多源搜索结果聚合

### 播放器
- 全屏播放器界面
- 专辑封面旋转动画
- 歌词显示
- 播放控制（播放/暂停、上一首/下一首）
- 播放模式切换
- 进度条拖动

### 播放列表
- 显示当前播放列表
- 删除歌曲
- 清空列表

### 设置
- 音质选择
- 歌词设置
- 缓存管理

## 构建

### 前置要求
- Android Studio Hedgehog (2023.1.1) 或更高版本
- JDK 17
- Android SDK 34
- Rust 工具链（用于构建核心库）

### 构建步骤

1. 构建 Rust 核心库：
```bash
cd ../rust-core
cargo ndk -t armeabi-v7a -t arm64-v8a -t x86_64 \
    -o ../android/app/src/main/jniLibs build --release
```

2. 打开 Android Studio 并同步 Gradle

3. 构建并运行应用

## FFI 通信

Android 层通过 JNI 与 Rust 核心通信：

1. **Kotlin 层**定义 Bridge 类（如 `PlayerBridge`）
2. 使用 `external` 关键字声明 native 方法
3. 在 `static` 块中加载 Rust 库：
   ```kotlin
   init {
       System.loadLibrary("lx_music_core")
   }
   ```
4. 数据通过 JSON 格式在 Kotlin 和 Rust 之间传递
5. Kotlin 使用 Kotlinx Serialization 进行序列化/反序列化

## 许可证
MIT
