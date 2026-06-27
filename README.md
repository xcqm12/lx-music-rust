<p align="center"><a href="https://github.com/xcqm12/lx-music-rust"><img width="200" src="doc/images/icon.png" alt="lx-music logo"></a></p>

<h1 align="center">LX Music 移动版 - Rust 重写</h1>

<p align="center">
  <a href="https://github.com/xcqm12/lx-music-rust/releases"><img src="https://img.shields.io/github/release/xcqm12/lx-music-rust" alt="Release version"></a>
  <a href="https://github.com/xcqm12/lx-music-rust/actions/workflows/release.yml"><img src="https://github.com/xcqm12/lx-music-rust/workflows/Build/badge.svg" alt="Build status"></a>
  <a href="https://github.com/xcqm12/lx-music-rust/actions/workflows/beta-pack.yml"><img src="https://github.com/xcqm12/lx-music-rust/workflows/Build%20Beta/badge.svg" alt="Build status"></a>
  <!-- <a href="https://github.com/xcqm12/lx-music-rust/releases"><img src="https://img.shields.io/github/downloads/xcqm12/lx-music-rust/latest/total" alt="Downloads"></a> -->
  <a href="https://github.com/xcqm12/lx-music-rust/blob/main/LICENSE"><img src="https://img.shields.io/github/license/xcqm12/lx-music-rust" alt="License"></a>
</p>

<p align="center">一个基于 React Native + Rust 开发的音乐软件</p>

## 说明

该项目是 [LX Music 移动版](https://github.com/lyswhut/lx-music-mobile) 的 Rust 重写版本，将核心音乐源解析、歌词处理、播放器等功能迁移至 Rust 实现。

### 技术栈

| 层级 | 技术 |
|------|------|
| UI | React Native 0.73 + TypeScript |
| 桥接 | TurboModule (RN 0.68+) |
| JNI | Kotlin → Rust FFI |
| 核心 | Rust (serde, reqwest, aes, hmac, Symphonia) |
| 构建 | Gradle 8.8 + CMake 3.18.1 + NDK 26 |

已支持的平台：

- Android 5 及以上

***注：目前没有计划支持 iOS 和 HarmonyOS NEXT**。*<br>
*原始项目地址：<https://github.com/lyswhut/lx-music-mobile>*<br>
*桌面版项目地址：<https://github.com/lyswhut/lx-music-desktop>*<br>

软件变化请查看[更新日志](https://github.com/xcqm12/lx-music-rust/blob/main/CHANGELOG.md)。

软件下载请查看 [GitHub Releases](https://github.com/xcqm12/lx-music-rust/releases)。

使用常见问题请参阅[移动版常见问题](https://lyswhut.github.io/lx-music-doc/mobile/faq)。

为了提高使用门槛，本软件内的默认设置、UI 操作不以新手友好为目标，所以使用前建议先根据你的喜好浏览调整一遍软件设置，阅读一遍[音乐播放列表机制](https://lyswhut.github.io/lx-music-doc/mobile/faq/playlist)。

### 更新说明

本项目在保留原 LX Music Mobile React Native UI 层的基础上，将核心业务逻辑迁移至 Rust 实现，主要变更：

- **音乐源解析**：酷我、酷狗、咪咕等音乐源解析逻辑由 JS 重写为 Rust（`music-source` crate），提升解析效率
- **歌词处理**：LRC/KRC 歌词解析、时间轴同步逻辑迁移至 Rust（`lyric` crate）
- **播放引擎**：音频引擎、解码器、播放列表管理迁移至 Rust（`player` crate），使用 Symphonia 进行音频解码
- **加密解密**：AES、HMAC、SHA2 等加密算法使用 Rust 原生实现，替代 JS 加密库
- **JNI 桥接**：通过 Rust FFI 与 Android Kotlin 层交互，使用 TurboModule 暴露接口给 React Native
- **安全文本**：SafeText 组件强制字符串化文本内容，规避 RN Text 组件校验报错

### Rust 核心模块

```
rust-core/
├── crates/
│   ├── common/        # 公共类型、错误处理、工具函数
│   ├── lyric/         # 歌词解析、同步管理
│   ├── music-source/  # 音乐源（酷我、酷狗、咪咕、QQ、网易）
│   ├── player/        # 音频引擎、解码器、播放列表
│   └── android/       # Android JNI 绑定
```

### 架构设计

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

### 构建指南

#### 环境准备

- **Node.js** >= 18, **npm** >= 8.5.2
- **Rust** (stable) + Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`)
- **Android Studio** + SDK 34 + NDK 26+
- **CMake** 3.18+

#### 构建步骤

**Windows (PowerShell):**

```powershell
# 1. 安装依赖
npm install

# 2. 设置 NDK 环境变量
$env:ANDROID_NDK_HOME = "$env:LOCALAPPDATA\Android\Sdk\ndk\<version>"

# 3. 构建 Rust 核心 (.so) - 推荐：一次性构建所有架构
cd rust-core/crates/android
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -t x86 -o ../jniLibs build --release

# 4. 复制 JNI 库到 Android 项目
Copy-Item -Recurse -Force "../jniLibs/*" "../../../android/app/src/main/jniLibs/"

# 5. 构建 Android APK
cd ../../../android
.\gradlew.bat assembleRelease --no-daemon
```

**Linux/macOS:**

```bash
# 1. 安装依赖
npm install

# 2. 设置 NDK 环境变量
export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/<version>

# 3. 构建 Rust 核心 (.so) - 推荐：一次性构建所有架构
cd rust-core/crates/android
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -t x86 -o ../jniLibs build --release

# 4. 复制 JNI 库到 Android 项目
cp -r ../jniLibs/* ../../../android/app/src/main/jniLibs/

# 5. 构建 Android APK
cd ../../../android
./gradlew assembleRelease --no-daemon
```

#### 调试运行

```bash
# 启动 Metro 开发服务器
npm start

# 安装并运行到设备
npx react-native run-android --active-arch-only

# 打开调试菜单
adb shell input keyevent 82
```

### 数据同步服务

从 v1.0.0 起，我们发布了一个独立的[数据同步服务](https://github.com/lyswhut/lx-music-sync-server#readme)。如果你有服务器，可以将其部署到服务器上作为私人多端同步服务使用，详情看该项目说明。

## 贡献代码

本项目欢迎 PR，但为了 PR 能顺利合并，需要注意以下几点：

- 对于添加新功能的 PR，建议在提交 PR 前先创建 Issue 进行说明，以确认该功能是否确实需要；
- 对于修复 bug 的 PR，请提供修复前后的说明及重现方式；
- 对于其他类型的 PR，则适当附上说明。

贡献代码步骤：

1. 参照[源码使用方法](https://lyswhut.github.io/lx-music-doc/mobile/use-source-code)设置开发环境；
2. 克隆本仓库代码并切换至 `dev` 分支进行开发；
3. 提交 PR 至 `dev` 分支。

## 项目协议

本项目基于 [Apache License 2.0](https://github.com/xcqm12/lx-music-rust/blob/main/LICENSE) 许可证发行，以下协议是对于 Apache License 2.0 的补充，如有冲突，以以下协议为准。

---

*词语约定：本协议中的"本项目"指 LX Music（洛雪音乐）移动版 Rust 重写项目；"使用者"指签署本协议的使用者；"官方音乐平台"指对本项目内置的包括酷我、酷狗、咪咕等音乐源的官方平台统称；"版权数据"指包括但不限于图像、音频、名字等在内的他人拥有所属版权的数据。*

### 一、数据来源

1.1 本项目的各官方平台在线数据来源原理是从其公开服务器中拉取数据（与未登录状态在官方平台 APP 获取的数据相同），经过对数据简单地筛选与合并后进行展示，因此本项目不对数据的合法性、准确性负责。

1.2 本项目本身没有获取某个音频数据的能力，本项目使用的在线音频数据来源来自软件设置内"自定义源"设置所选择的"源"返回的在线链接。例如播放某首歌，本项目所做的只是将希望播放的歌曲名、艺术家等信息传递给"源"，若"源"返回了一个链接，则本项目将认为这就是该歌曲的音频数据而进行使用，至于这是不是正确的音频数据本项目无法校验其准确性，所以使用本项目的过程中可能会出现希望播放的音频与实际播放的音频不对应或者无法播放的问题。

1.3 本项目的非官方平台数据（例如"我的列表"内列表）来自使用者本地系统或者使用者连接的同步服务，本项目不对这些数据的合法性、准确性负责。

### 二、版权数据

2.1 使用本项目的过程中可能会产生版权数据。对于这些版权数据，本项目不拥有它们的所有权。为了避免侵权，使用者务必在 **24 小时内** 清除使用本项目的过程中所产生的版权数据。

### 三、音乐平台别名

3.1 本项目内的官方音乐平台别名为本项目内对官方音乐平台的一个称呼，不包含恶意。如果官方音乐平台觉得不妥，可联系本项目更改或移除。

### 四、资源使用

4.1 本项目内使用的部分包括但不限于字体、图片等资源来源于互联网。如果出现侵权可联系本项目移除。

### 五、免责声明

5.1 由于使用本项目产生的包括由于本协议或由于使用或无法使用本项目而引起的任何性质的任何直接、间接、特殊、偶然或结果性损害（包括但不限于因商誉损失、停工、计算机故障或故障引起的损害赔偿，或任何及所有其他商业损害或损失）由使用者负责。

### 六、使用限制

6.1 本项目完全免费，且开源发布于 GitHub 面向全世界人用作对技术的学习交流。本项目不对项目内的技术可能存在违反当地法律法规的行为作保证。

6.2 **禁止在违反当地法律法规的情况下使用本项目。** 对于使用者在明知或不知当地法律法规不允许的情况下使用本项目所造成的任何违法违规行为由使用者承担，本项目不承担由此造成的任何直接、间接、特殊、偶然或结果性责任。

### 七、版权保护

7.1 音乐平台不易，请尊重版权，支持正版。

### 八、非商业性质

8.1 本项目仅用于对技术可行性的探索及研究，不接受任何商业（包括但不限于广告等）合作及捐赠。

### 九、接受协议

9.1 若你使用了本项目，即代表你接受本协议。