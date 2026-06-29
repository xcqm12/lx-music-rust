修复 React Native 运行时崩溃问题，完善调试环境配置，添加 ESLint 规则防止 Text 组件渲染错误。

### 修复

- **Text 组件崩溃**：创建 SafeText 组件，通过 `toSafeString` 函数强制转换 null/undefined 等非法值为空字符串，避免 RN Text 组件渲染报错
- **TrackPlayer 参数错误**：修复 `updateNowPlayingMetadata` 调用缺少 `isPlaying` 参数导致的 NativeArgumentsParseException 崩溃
- **react-native-track-player 依赖**：补充缺失的 `lib/` 编译产物，解决 Metro bundler 模块解析失败
- **列表渲染安全**：修复 `ListItem`、`CommentText`、`Header` 等组件中可能导致 null/undefined 渲染的问题

### 构建

- 完善 ADB 端口转发配置，支持 MuMu 模拟器调试
- 优化 Metro bundler 缓存重置流程，解决模块解析缓存问题

### 代码质量

- **ESLint 配置**：添加 `eslint-plugin-react-native` 插件，启用 `react-native/no-raw-text` 规则，强制检查所有文本是否在 `<Text>` 组件内渲染，从开发阶段杜绝 Text 组件崩溃问题