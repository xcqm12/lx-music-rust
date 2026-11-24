// 车机场景无法抓 logcat，错误日志自动上报到本 fork 仓库的 GitHub Issues，
// 让用户无需手动复制日志即可反馈问题。日志可能包含用户操作记录，
// 可在 设置-错误日志 中关闭自动上传。
//
// 双通道：优先走自建 SSH 中转（大陆网络可达；GitHub 凭证只存服务端，不进 APK），
// 中转不可用时回退直连 api.github.com（APK 内混淆凭证或用户覆盖凭证）。
import { Platform } from 'react-native'
import { httpFetch } from '@/utils/request'
import { getSupportedAbis, getMemoryInfo, reportViaSsh } from '@/utils/nativeModules/utils'
import { getData, saveData, removeData } from '@/plugins/storage'
import { setLogErrorListener, getLogs } from '@/utils/log'
import settingState from '@/store/setting/state'
import decodeReportToken from '@/config/reportToken'
import getRelayConfig from '@/config/reportRelay'

// 上传目标必须是本 fork 仓库，不可回退到上游 lyswhut/lx-music-mobile
const REPO = 'xcqm12/lx-music-rust'
const ISSUES_API = `https://api.github.com/repos/${REPO}/issues`

const TOKEN_KEY = '@error_report_token'
const STATE_KEY = '@error_report_state'

const MAX_LOG_LENGTH = 48000 // issue body 上限 65536，留余量
const COALESCE_DELAY = 15000 // 多条错误合并上报的窗口
const STARTUP_DELAY = 30000 // 启动检查延迟，给 CrashSaver 崩溃日志合并留时间
const MIN_UPLOAD_INTERVAL = 6 * 60 * 60 * 1000 // 两次成功上传的最小间隔
const MIN_ATTEMPT_INTERVAL = 30 * 60 * 1000 // 两次尝试（含失败）的最小间隔

type ReportTrigger = 'startup' | 'error' | 'manual'

const TRIGGER_NAMES: Record<ReportTrigger, string> = {
  startup: '启动检查',
  error: '错误触发',
  manual: '手动上传',
}

interface ReportState {
  lastUploadAt?: number
  lastAttemptAt?: number
  lastLogHash?: string
  /** 累计 log.error 条数（含历史，持久化避免重启清零） */
  errorCountTotal?: number
  /** 上次成功上传时的 errorCountTotal 快照——无新错误则启动检查不再重复上报陈旧日志 */
  lastUploadedErrorCount?: number
}

interface UploadResult {
  ok: boolean
  message: string
  /** 已明确是限流/重复等业务原因，不必再尝试其他通道 */
  skipFallback?: boolean
}

let coalesceTimer: ReturnType<typeof setTimeout> | null = null
let uploading = false
let cachedToken: string | null = null
/** 本次运行累计 log.error 条数；initErrorReport 时从持久化状态续接 */
let errorCount = 0

const isEnabled = () => {
  // 未初始化完成时视为开启（默认开启）
  const value = settingState.setting['common.isAutoUploadErrorLog'] as boolean | undefined
  return value ?? true
}

const resolveToken = async() => {
  if (cachedToken !== null) return cachedToken
  let token = (await getData<string>(TOKEN_KEY)) ?? ''
  if (!token) token = decodeReportToken()
  // eslint-disable-next-line require-atomic-updates
  cachedToken = token
  return token
}

/** 设置页手动覆盖内置凭证；传空字符串清除覆盖 */
export const setErrorReportToken = async(token: string) => {
  cachedToken = null
  const value = token.trim()
  if (value) {
    await saveData(TOKEN_KEY, value)
  } else {
    await removeData(TOKEN_KEY)
  }
}

export const getErrorReportToken = async() => {
  return (await getData<string>(TOKEN_KEY)) ?? ''
}

const loadState = async(): Promise<ReportState> => {
  return await getData<ReportState>(STATE_KEY) ?? {}
}

const saveState = async(state: ReportState) => {
  await saveData(STATE_KEY, state)
}

// FNV-1a：识别日志是否变化足够了，无需加密级哈希
const hashLog = (text: string) => {
  let hash = 0x811c9dc5
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i)
    hash = Math.imul(hash, 0x01000193) >>> 0
  }
  return hash.toString(16)
}

// 是否包含真实错误内容：ERROR 行 / 崩溃 / 疑似 ANR / 异常退出。
// 面包屑（app.start、memory.trim）与轮转标记属于运行信息——低内存车机上 trim 是常态，
// 若仅因日志非空就在启动检查时上报，会持续产生“未知错误”噪音 issue。
const hasRealError = (log: string) => {
  if (log.includes('[CRASH]') || log.includes('[anr]') || log.includes('[exit]')) return true
  return log.split('\n').some(line => line.includes(' ERROR '))
}

const getAbi = async() => {
  try {
    return (await getSupportedAbis())[0] ?? ''
  } catch {
    return ''
  }
}

const getDeviceName = () => {
  try {
    const constants = Platform.constants as { Manufacturer?: string, Model?: string }
    return [constants.Manufacturer, constants.Model].filter(Boolean).join(' ')
  } catch {
    return ''
  }
}

// 取最后一条 ERROR 行" ERROR "之后的内容作为标题摘要；纯崩溃日志则标记为原生崩溃。
// 用 split 而非 ^\S+\s 前缀剔除——时间戳是本地化格式（"2026年11月24日星期二 中国标准时间 上午7:50:10"），
// 含空格，单前缀正则剔不干净会把时间戳带进 issue 标题。
const buildSummary = (log: string) => {
  const lines = log.split('\n')
  for (let i = lines.length - 1; i >= 0; i--) {
    if (!lines[i].includes(' ERROR ')) continue
    const summary = lines[i].split(' ERROR ').pop()?.trim() ?? ''
    if (summary) return summary.slice(0, 80)
  }
  if (log.includes('[CRASH]')) return '原生崩溃'
  return '未知错误'
}

const buildBody = async(trigger: ReportTrigger, log: string, truncated: boolean) => {
  const abi = await getAbi()
  const device = getDeviceName()
  const memory = await getMemoryInfo()
  const memoryText = memory?.totalMemMB
    ? `- 内存：可用 ${memory.availMemMB ?? '?'}MB / 总共 ${memory.totalMemMB}MB${memory.lowMemory ? '（系统低内存）' : ''}，应用堆 ${memory.heapUsedMB ?? '?'}/${memory.heapMaxMB ?? '?'}MB，存储剩余 ${memory.diskFreeMB ?? '?'}MB`
    : null
  const lines = [
    '<!-- 本 issue 由应用自动上报（设置-错误日志），由维护者处理 -->',
    '',
    '## 环境信息',
    `- 应用版本：${process.versions?.app ?? 'unknown'}`,
    `- 系统：Android API ${Platform.Version}`,
    device ? `- 设备：${device}` : null,
    abi ? `- ABI：${abi}` : null,
    memoryText,
    `- 上报时间：${new Date().toLocaleString()}`,
    `- 触发方式：${TRIGGER_NAMES[trigger]}`,
    '',
    '## 错误日志',
    '',
  ].filter(Boolean)
  if (truncated) lines.push(`（日志超过 ${MAX_LOG_LENGTH} 字符，仅保留最后部分）`, '')
  lines.push('```text', log, '```')
  return lines.join('\n')
}

/** HTTPS 中转上传（443）：车机网络常封禁非标 SSH 端口，优先走这里。
 *  服务端 nginx 反代到本地受限服务，校验固定令牌后复用 report-submit。 */
const uploadViaHttp = async(title: string, body: string): Promise<UploadResult> => {
  const relay = getRelayConfig()
  if (!relay?.httpUrl || !relay.httpToken) return { ok: false, message: 'HTTPS 中转未配置' }
  try {
    const { promise } = httpFetch(relay.httpUrl, {
      method: 'post',
      timeout: 20000,
      headers: {
        'Content-Type': 'application/json',
        'X-LX-Report-Token': relay.httpToken,
      },
      // 交给 request.js 统一 JSON.stringify（直接传字符串会被二次编码）
      body: { title, body },
    })
    const resp = await promise
    const text = (typeof resp.body == 'string' ? resp.body : '').trim()
    if (text.startsWith('OK')) return { ok: true, message: '上传成功，感谢反馈' }
    if (text.startsWith('DUP')) return { ok: true, message: '该日志已上报过，无需重复上传' }
    if (text.includes('ERR rate')) {
      console.log('http relay rate limited:', resp.statusCode, text)
      return { ok: false, skipFallback: true, message: '提交过于频繁，请稍后再试' }
    }
    console.log('http relay failed:', resp.statusCode, text)
    return { ok: false, message: `HTTPS 中转失败（${text || `HTTP ${resp.statusCode}`}）` }
  } catch (err) {
    console.log('http relay error:', (err as Error).message)
    return { ok: false, message: `HTTPS 中转连接失败（${(err as Error).message}）` }
  }
}

/** SSH 中转上传。服务端协议：stdin 为 JSON {title, body}，
 *  退出码 0 且输出 OK <url> 为成功，DUP 为重复（已上报过），ERR* 为失败。
 *  注意：部分设备/网络下远端退出码会读不到（code = -1），此时以输出协议为准，
 *  避免“服务端已建 issue 但 APP 判为失败并重复提交”。
 *  连接信息与受限密钥来自混淆的 reportRelay.ts（host key 指纹由原生层校验防 MITM）。 */
const uploadViaRelay = async(title: string, body: string): Promise<UploadResult> => {
  const relay = getRelayConfig()
  if (!relay) return { ok: false, message: '中转未配置' }
  try {
    const { code, output, stderr } = await reportViaSsh({
      host: relay.host,
      port: relay.port,
      username: relay.user,
      hostKeySha256: relay.hostKeySha256,
      privateKey: relay.key,
      timeout: 25000,
    }, JSON.stringify({ title, body }))
    const text = (output ?? '').trim()
    if (text.startsWith('OK')) return { ok: true, message: '上传成功，感谢反馈' }
    if (text.startsWith('DUP')) return { ok: true, message: '该日志已上报过，无需重复上传' }
    if (text.includes('ERR rate')) {
      console.log('relay rate limited:', code, text)
      return { ok: false, skipFallback: true, message: '提交过于频繁，请稍后再试' }
    }
    if (text.startsWith('ERR')) {
      console.log('relay upload failed:', code, text, stderr)
      return { ok: false, message: `中转上传失败（${text}）` }
    }
    // 输出丢失但远端命令正常结束，按成功处理（服务端有内容去重，不会重复建 issue）
    if (code === 0) return { ok: true, message: '上传成功，感谢反馈' }
    console.log('relay upload failed:', code, text, stderr)
    const reason = [text, stderr ?? ''].find(value => value.length > 0) ?? `exit ${code}`
    return { ok: false, message: `中转上传失败（${reason}）` }
  } catch (err) {
    console.log('relay connect failed:', (err as Error).message)
    return { ok: false, message: `中转连接失败（${(err as Error).message}）` }
  }
}

/** 直连 GitHub API（混淆内置凭证或用户覆盖凭证） */
const uploadDirect = async(title: string, body: string): Promise<UploadResult> => {
  const token = await resolveToken()
  if (!token) return { ok: false, message: '未配置上传凭证，请填写 Token' }
  try {
    const { promise } = httpFetch(ISSUES_API, {
      method: 'post',
      timeout: 20000,
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
      },
      body: { title, body, labels: ['auto-report'] },
    })
    const resp = await promise
    if (resp.statusCode === 201) {
      console.log('error report direct uploaded, status:', resp.statusCode)
      return { ok: true, message: '上传成功，感谢反馈' }
    }
    const reason = (resp.body as { message?: string } | undefined)?.message ?? String(resp.statusCode)
    console.log('direct upload failed:', resp.statusCode, reason)
    return { ok: false, message: `直连上传失败（${resp.statusCode} ${reason}）` }
  } catch (err) {
    console.log('direct upload error:', (err as Error).message)
    return { ok: false, message: `直连上传失败（${(err as Error).message}）` }
  }
}

const upload = async(trigger: ReportTrigger): Promise<UploadResult> => {
  if (!isEnabled()) return { ok: false, message: '自动上传已关闭' }
  if (uploading) return { ok: false, message: '正在上传中，请稍候' }

  const log = await getLogs()
  if (!log?.trim()) return { ok: false, message: '日志为空，无需上传' }
  // 启动检查/错误触发只在有实际错误时上报；手动上传尊重用户意图，不做此过滤
  if (trigger !== 'manual' && !hasRealError(log)) return { ok: false, message: '日志无实际错误内容，已跳过' }

  const truncated = log.length > MAX_LOG_LENGTH
  const content = truncated ? log.slice(-MAX_LOG_LENGTH) : log
  const contentHash = hashLog(content.slice(-4000))

  const state = await loadState()
  const now = Date.now()
  if (trigger !== 'manual') {
    if (state.lastAttemptAt && now - state.lastAttemptAt < MIN_ATTEMPT_INTERVAL) return { ok: false, message: '距上次尝试过近，已跳过' }
    if (state.lastUploadAt && now - state.lastUploadAt < MIN_UPLOAD_INTERVAL) return { ok: false, message: '距上次上传不足 6 小时，已跳过' }
    if (contentHash === state.lastLogHash) return { ok: false, message: '日志无变化，已跳过' }
    // 上次上报后没有新 ERROR：日志尾部的哈希变化可能只是 INFO 滚动，
    // 不能证明出现了新问题——不再重复上报陈旧错误（启动检查 stale 报告的根因）
    if (state.lastUploadedErrorCount != null && errorCount === state.lastUploadedErrorCount) {
      return { ok: false, message: '上次上报后无新错误，已跳过' }
    }
    state.lastAttemptAt = now
    await saveState(state)
  }

  // eslint-disable-next-line require-atomic-updates
  uploading = true
  try {
    const title = `[自动错误上报] v${process.versions?.app ?? ''} API${Platform.Version} ${await getAbi()} ${buildSummary(log)}`.trim().slice(0, 200)
    const body = await buildBody(trigger, content, truncated)

    // 通道优先级：HTTPS(443) → SSH(自建端口) → 直连 GitHub。
    // 车机网络常见封禁非标 SSH 端口，HTTPS 走站点同域更可靠；限流/重复不再无谓重试。
    const attempts: UploadResult[] = []
    let result: UploadResult = { ok: false, message: '' }
    for (const channel of [uploadViaHttp, uploadViaRelay, uploadDirect]) {
      const channelResult = await channel(title, body)
      if (channelResult.ok) {
        result = channelResult
        break
      }
      attempts.push(channelResult)
      result = channelResult
      if (channelResult.skipFallback) break
    }
    if (!result.ok && attempts.length > 1) {
      result = {
        ok: false,
        message: attempts.map(item => item.message).join('；'),
      }
    }

    if (result.ok) {
      const newState = await loadState()
      newState.lastUploadAt = Date.now()
      newState.lastLogHash = contentHash
      newState.lastUploadedErrorCount = errorCount
      await saveState(newState)
    }
    return result
  } finally {
    // eslint-disable-next-line require-atomic-updates
    uploading = false
  }
}

// log.error 钩子：计数 + 错误发生后延迟合并窗口再上传，避免一次异常刷多条
const notifyError = (_msg: string) => {
  if (!isEnabled()) return
  errorCount++
  void loadState().then(async state => {
    state.errorCountTotal = errorCount
    await saveState(state)
  }).catch(() => { /* 持久化失败不影响计数 */ })
  if (coalesceTimer) clearTimeout(coalesceTimer)
  coalesceTimer = setTimeout(() => {
    coalesceTimer = null
    void upload('error')
  }, COALESCE_DELAY)
}

export const uploadNow = async(): Promise<UploadResult> => {
  return upload('manual')
}

/** 注册错误监听并在启动后做一次节流检查（原生崩溃日志此时已并入 error.log） */
export const initErrorReport = () => {
  // 续接累计错误计数（重启后从持久化状态恢复，避免启动检查把历史错误当新错误）
  void loadState().then(state => {
    errorCount = state.errorCountTotal ?? 0
  }).catch(() => { /* 保持 0 */ })
  setLogErrorListener(notifyError)
  setTimeout(() => {
    void upload('startup')
  }, STARTUP_DELAY)
}
