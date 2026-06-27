# 使用 Rust 重写 LX Music Mobile 项目指南

本指南说明如何使用 Rust 重写项目的核心功能，添加自定义音源 JS 文件支持，并打包成 Android 应用。

## 目录结构

```
lx-music-mobile-master/
├── rust-lib/                    # Rust JS 引擎库
│   ├── Cargo.toml               # Rust 项目配置
│   ├── src/
│   │   ├── lib.rs              # 主入口
│   │   ├── js_engine.rs        # JS 执行引擎
│   │   ├── music_source.rs     # 音源数据结构
│   │   └── jni_bindings.rs     # JNI 导出
│   ├── android-libs/            # 预编译的 Android 库
│   ├── build.ps1                # Windows 构建脚本
│   └── README.md               # 详细文档
├── android/                     # Android 原生代码
│   └── app/src/main/java/cn/toside/music/mobile/
│       ├── RustBridgeModule.java    # React Native 桥接模块
│       └── RustBridgePackage.java   # Package 注册
└── src/                          # React Native 代码
    └── utils/rust/
        └── index.ts              # TypeScript 桥接
    └── screens/Home/Views/Setting/settings/CustomSource/
        └── index.tsx             # 自定义音源管理 UI
```

## 构建步骤

### 1. 安装依赖

确保已安装以下工具：

- **Rust**: https://rustup.rs/
- **cargo-ndk**: `cargo install cargo-ndk`
- **Android NDK**: 通过 Android Studio 自动安装

### 2. 构建 Rust 库

```powershell
# Windows
cd rust-lib
.\build.ps1

# 或手动构建
cargo ndk -l <ndk-version> build --release
```

### 3. 构建 Android APK

```bash
cd android
./gradlew assembleDebug
# 或打包正式版
./gradlew assembleRelease
```

## 功能说明

### 1. Rust JS 引擎 (rquickjs)

使用 `rquickjs` crate 来执行 JavaScript 代码，这是一个轻量级高效的 JS 引擎：

- 支持 ES2020 特性
- 垃圾回收机制
- 小型二进制文件

### 2. 自定义音源 JS 文件

用户可以添加自定义音源 JS 文件，实现以下功能：

```javascript
// 示例：my_source.js
module.exports = {
  search: async (keyword) => {
    // 实现搜索逻辑
    return [
      { id: '1', name: '歌曲名', singer: '歌手', source: 'my_source', duration: '03:45' }
    ]
  },
  getMusicInfo: async (musicId) => {
    // 获取歌曲详情
    return { id: musicId, name: '歌曲名', singer: '歌手', albumName: '专辑' }
  },
  getLyric: async (musicId) => {
    // 获取歌词
    return { lyric: '[00:00.00]歌词内容' }
  },
  getUrl: async (musicId, quality) => {
    // 获取播放链接
    return { url: 'https://example.com/music.mp3' }
  }
}
```

### 3. UI 管理界面

在设置页面添加了"自定义音源"管理界面：

- 添加新音源
- 编辑现有音源
- 删除音源
- 测试音源功能

## 打包成功检查清单

- [x] Rust 库编译成功 (`liblex_music_rust.so`)
- [x] Android CMake 配置正确
- [x] React Native Native Module 已注册
- [x] UI 界面已添加
- [x] 多语言翻译已添加

## 常见问题

### Q: 编译时报错 "library not found"

确保先运行 Rust 构建脚本，生成了 `rust-lib/android-libs/liblex_music_rust.so`

### Q: APK 启动时报错 "UnsatisfiedLinkError"

检查 APK 中是否包含 `liblx_music_rust.so` 文件，或检查 CMakeLists.txt 配置

### Q: JS 引擎初始化失败

查看 Logcat 中的 Rust 引擎日志输出

## 后续优化建议

1. **性能优化**: 考虑使用 Rust 的异步运行时（如 Tokio）处理并发请求
2. **缓存机制**: 添加音源搜索结果缓存
3. **错误处理**: 增强错误处理和用户提示
4. **安全沙箱**: 增强 JS 代码执行的安全性隔离
