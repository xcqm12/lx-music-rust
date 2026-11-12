# 车机版错误日志自动上传 GitHub Issues

## Context

本 fork（xcqm12/lx-music-rust）整体即车机版（Android 5+ 低内存车机），用户无法抓 logcat，反馈问题全靠手动截屏发 issue。现有错误链路已完备：JS 异常（`src/utils/errorHandle.ts` → `log.error`）+ 原生崩溃（`CrashSaver.kt` 同步落盘 `filesDir/crash/`，下次启动由 `mergeCrashLogs()` 并入 `CacheDir/error.log`），但**没有任何上传能力**。

目标：错误发生时自动把 error.log 上传为 https://github.com/xcqm12/lx-music-rust 的 issue。

用户已确认：**默认开启**；触发策略为**错误发生即传 + 节流去重**；token 采用**两者结合**——源码内置混淆加密的默认 token（用户明确要求写进公开源码并做强混淆/去符号），构建期 CI secret 可覆盖，设置页可手动覆盖。

## Token 方案（含诚实的风险说明）

- 新脚本 `scripts/gen-report-token.mjs`：输入原始 PAT（env/argv），输出混淆文件 `src/config/reportToken.ts`。混淆 = UTF-8 字节 XOR 32 字节密钥（密钥拆两半存文件）→ base64 → 切 5 块乱序 + 索引表，变量名单字母化，文件内不出现 token/github/ghp_ 等可扫描符号；脚本自带 roundtrip 自测。
- **必须明确：混淆只防自动扫描（GitHub push protection、爬虫正则、低成本脚本），挡不住有意逆向**——APK 运行时必然还原 token。真正的止损是：细粒度 PAT 仅授该仓库 Issues:write 权限 + 客户端节流。若发现滥用可随时换 token 重新生成文件。
- 构建期覆盖：CI 里 `node scripts/gen-report-token.mjs --env`（读 secret `LX_REPORT_TOKEN`）重新生成该文件后再打包，secret 缺省则用已提交文件（沿用现有签名 secret 的判空模式）。

## 实现

### 1. 新建 `src/utils/errorReport.ts`（核心模块）
- `const REPO = 'xcqm12/lx-music-rust'`（对齐 `src/utils/version.js:17` 的注释风格，说明不可回退上游）。
- token 解析优先级：AsyncStorage 覆盖值（`saveData('error_report_token_override', …)`）→ `src/config/reportToken.ts` 解码值 → 空（功能禁用）。
- 状态存 AsyncStorage key `@error_report_state`：`{ lastUploadAt, lastAttemptAt, lastLogHash }`（用 `@/plugins/storage` 的 `getData/saveData`）。
- `notifyError(msg)`：由 log.error 触发，15s 合并窗口后尝试上传；`checkStartup()`：启动 30s 后检查（给原生崩溃合并留时间）；`uploadNow()`：设置页手动触发，跳过节流但仍去重可选。
- 节流：两次成功上传 ≥6h；两次尝试 ≥30min；日志 hash 未变则跳过（手动上传强制执行）。
- payload：`getLogs()` 超 48000 字符取尾部（最新条目在末尾）+ 截断标记；标题 `[自动错误上报] v{process.versions.app} API{Platform.Version} {abi} {摘要≤80字}`（摘要取最后一条 ERROR/[CRASH] 首行）；正文含环境信息（版本/API/ABI/设备型号取 `Platform.constants` 需 try 包裹）+ 日志围栏代码块。
- 上传走 `httpFetch`（`src/utils/request.js`，JSON POST 已支持自定义 headers）：`Authorization: Bearer …`、`Accept: application/vnd.github+json`、`X-GitHub-Api-Version: 2022-11-28`，body `{ title, body, labels: ['auto-report'] }`（不存在的 label GitHub 静默丢弃，无害）。201 视为成功。任何错误输出/日志**不得包含 token**。

### 2. `src/utils/log.ts` 挂钩（无环依赖）
导出 `setLogErrorListener(fn)`，`log.error` 末尾 try/catch 调用 `listener?.(msg)`。errorReport 单向 import log.ts，无循环。

### 3. `src/core/init/index.ts` 启动接线
`dataInit` 之后 `void initErrorReport()`（注册监听 + 启动检查；在 `app.ts` 的 initLog 之后，时序安全）。

### 4. 设置项与 UI
- `src/config/defaultSetting.ts` + `src/types/app_setting.d.ts`：新增 `'common.isAutoUploadErrorLog': boolean`，默认 `true`。
- `src/screens/Home/Views/Setting/settings/Other/Log.tsx`：CheckBoxItem 开关（走 `useSetting`/`updateSetting` 持久化）、「立即上传」按钮（toast 结果）、「上传 Token」按钮（ConfirmAlert + TextInput，可设置/清空覆盖值）。
- i18n 三份：`src/lang/zh-cn.json`、`zh-tw.json`、`en-us.json` 新增约 8 个 `setting_other_log_*` 键。

### 5. CI（`.github/workflows/release.yml`）
Build 步骤前加（沿用签名 secret 判空守卫风格）：
```yaml
- if: ${{ secrets.LX_REPORT_TOKEN != '' }}
  run: node scripts/gen-report-token.mjs --env
```

## 验证

1. `node scripts/gen-report-token.mjs` 自测 roundtrip 通过；用 dummy token 重新生成 `reportToken.ts` 并 `node -e` 验证解码还原。
2. `npx tsc --noEmit`（若有 tsconfig）+ `npm run lint` 通过。
3. `npm run build-test` 确认 bundle 可打包。
4. 真机端到端（需要你提供 PAT）：设置页开开关 → 「立即上传」→ GitHub 出现带日志的 issue；制造原生崩溃重启 → 自动建 issue；6h 内重复错误不再建；关闭开关后不再上传；清空 token 覆盖值回退默认。

## 不做

- Rust 侧（jni_bridge panic）仍只进 logcat，不接入本链路（改 JNI 桥成本高，另立任务）。
- 不做代理服务器/匿名上报（GitHub API 硬性要求认证）。