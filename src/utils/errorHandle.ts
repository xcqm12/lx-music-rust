import { Alert, Platform } from 'react-native'
// import { exitApp } from '@/utils/common'
import { setJSExceptionHandler, setNativeExceptionHandler } from 'react-native-exception-handler'
import { log } from '@/utils/log'
import { toast } from './tools'

const errorHandler = (e: Error, isFatal: boolean) => {
  const excludedErrors = [
    'Failed to construct \'Response\'',
  ]
  if (isFatal) {
    if (excludedErrors.some((excludedError) => e.message.includes(excludedError))) {
      toast('应用遇到了错误，如果你有固定的复现方式，请截图并在 GitHub 反馈（并附上具体的操作步骤，以及“设置-错误日志”的内容）')
    } else {
      Alert.alert(
        '💥Unexpected error occurred💥',
        `
  应用出 bug 了😭，以下是错误异常信息。请截图并在 GitHub 反馈（并附上刚才你进行了什么操作，以及附上“设置-错误日志”的内容）。现在应用可能会出现异常，若出现异常请尝试强制结束应用后重新启动！

  Error:
  ${isFatal ? 'Fatal:' : ''} ${e.name} ${e.message}
  `,
        [{
          text: '关闭 (Close)',
          onPress: () => {
            // exitApp()
          },
        }],
      )
    }
  }
  log.error(e.stack)
}

// 网络类拒绝是车机离线/弱网的常态（issue #15/#16：一次断网，十几个并行请求同时超时，
// 逐条 ERROR 刷爆错误日志并触发自动上报），记为 WARN——保留线索但不触发自动上报
// （errorReport 只监听 log.error），配合下方 30s 去重，一段断网只留一条。
const NETWORK_REJECTION_PATTERNS = [
  '请求超时',
  '无法连接到服务器',
  '接口无法访问',
  '取消http请求',
  '服务器繁忙',
  'socket hang up',
  'Network request failed',
  'Aborted',
  'timed out',
  'timeout',
]

const isNetworkRejection = (text: string) => NETWORK_REJECTION_PATTERNS.some(p => text.includes(p))

// 30s 窗口内相同文本的 rejection 只记首条，重复只累加计数，下次写入时以 (+N in 30s) 标注
const REJECTION_SUPPRESS_WINDOW = 30_000
let lastRejectionText = ''
let lastRejectionAt = 0
let suppressedRejectionCount = 0

const handleUnhandledRejection = (reason: unknown) => {
  const text = reason instanceof Error ? (reason.stack ?? `${reason.name}: ${reason.message}`) : String(reason)
  const now = Date.now()
  if (text === lastRejectionText && now - lastRejectionAt < REJECTION_SUPPRESS_WINDOW) {
    suppressedRejectionCount++
    lastRejectionAt = now
    return
  }
  const isRepeat = text === lastRejectionText
  const note = isRepeat && suppressedRejectionCount > 0
    ? ` (+${suppressedRejectionCount} in 30s)`
    : ''
  suppressedRejectionCount = 0
  lastRejectionText = text
  lastRejectionAt = now
  const line = `[UNHANDLED REJECTION] ${text}${note}`
  if (isNetworkRejection(text)) {
    log.warn(line)
  } else {
    log.error(line)
  }
}

if (process.env.NODE_ENV !== 'development') {
  setJSExceptionHandler(errorHandler)

  setNativeExceptionHandler((errorString) => {
    // 设备信息尽力而为地记入日志（原生崩溃时 JS 落盘不一定来得及，CrashSaver 负责同步兜底）
    const deviceInfo = `app=${process.versions?.app ?? ''} api=${Platform.Version} ${new Date().toLocaleString()}`
    log.error(`[NATIVE CRASH] ${deviceInfo}\n${errorString}`)
    console.log('+++++', errorString, '+++++')
  }, false)

  // Hermes 未处理的 Promise 拒绝：异步搜索/请求中的异常不再静默丢失
  try {
    const hermes = (global as { HermesInternal?: {
      enablePromiseRejectionTracker?: (options: {
        allRejections: boolean
        onUnhandled: (id: number, reason: unknown) => void
        onHandled: (id: number) => void
      }) => void
    } }).HermesInternal
    hermes?.enablePromiseRejectionTracker?.({
      allRejections: true,
      onUnhandled: (_id, reason) => {
        handleUnhandledRejection(reason)
      },
      onHandled: () => {},
    })
  } catch (err) {
    console.log('enable promise rejection tracker failed:', err)
  }
}
