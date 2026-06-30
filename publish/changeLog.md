React Native 文本渲染稳定性优化，修复多个潜在崩溃问题。

### 修复

- **Text 组件崩溃**：修复 "Text strings must be rendered within a <Text> component" 崩溃问题，通过 SafeText 组件自动处理空值和非字符串类型
- **设置页面显示异常**：修复设置页面显示 `[object Object]` 的问题，重构 `toSafeChildren` 函数保留 React 元素类型
- **TrackPlayer 参数错误**：修复 `updateNowPlayingMetadata` 调用缺少 `isPlaying` 参数导致的崩溃
- **移除自定义音源管理**：从设置页面移除自定义音源管理模块，清理相关路由和渲染逻辑

### 优化

- **ESLint 规则**：配置 `react-native/no-raw-text` 规则，强制检查文本渲染，从根本上避免裸文本导致的崩溃
- **Button 组件**：优化通用 Button 组件，自动用 Text 包裹字符串类型的 children
- **文本安全处理**：`toSafeChildren` 函数支持 React 元素、数组、基本类型等多种 children 类型

### 构建

- 完成正式版签名打包配置，支持 arm64-v8a、armeabi-v7a、x86_64、x86 四架构