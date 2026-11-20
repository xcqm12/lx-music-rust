// import { requestStoragePermission } from '@/utils/common'
import { temporaryDirectoryPath, privateStorageDirectoryPath, existsFile, appendFile, unlink, writeFile, readFile, readDir, stat } from '@/utils/fs'

const logPath = temporaryDirectoryPath + '/error.log'
// CrashSaver.kt 写入的原生崩溃目录（filesDir/crash）
const crashDirPath = privateStorageDirectoryPath + '/crash'

// 日志上限：2G 内存车机整读数 MB 日志即可 OOM，超限轮转保留尾部
const MAX_LOG_FILE_BYTES = 1024 * 1024
const KEEP_LOG_TAIL_BYTES = 256 * 1024

const logTools = {
  tempLog: [] as Array<{ time: string, type: 'LOG' | 'WARN' | 'ERROR', text: string }> | null,
  writeLog(msg: string) {
    console.log(msg)
    void appendFile(logPath, '\n----lx log----\n' + msg)
  },
  async initLogFile() {
    try {
      let isExists = await existsFile(logPath)
      // console.log(isExists)
      if (!isExists) await writeFile(logPath, '')
      await rotateLogFile()
      await mergeCrashLogs()
      await mergeBreadcrumbs()
      if (this.tempLog?.length) this.writeLog(this.tempLog.map(m => `${m.time} ${m.type} ${m.text}`).join('\n----lx log----\n'))
      this.tempLog = null
    } catch (err) {
      console.log(err)
    }
  },
}

/**
 * error.log 超过上限时轮转：只保留尾部内容重写文件。
 * 长期开机且频繁出错的车机，日志可涨到数十 MB，整读即 OOM。
 */
async function rotateLogFile() {
  try {
    const info = await stat(logPath)
    const size = Number(info?.size ?? 0)
    if (!size || size <= MAX_LOG_FILE_BYTES) return
    const content = await readFile(logPath)
    const tail = content.length > KEEP_LOG_TAIL_BYTES
      ? content.slice(content.indexOf('\n----lx log----\n', content.length - KEEP_LOG_TAIL_BYTES))
      : content
    await writeFile(logPath, '[LOG ROTATED] 旧日志已截断，仅保留尾部\n----lx log----\n' + tail.trimStart())
    console.log(`[log] rotated error.log: ${size} -> ${(await stat(logPath)).size} bytes`)
  } catch (err) {
    // 轮转失败不影响原文件
    console.log('rotate log failed:', err)
  }
}

/**
 * 把原生层面包屑（Diagnostics 落盘的 filesDir/diagnostics/breadcrumbs.log）并入 error.log。
 * 原生崩溃/OOM 被杀时 JS 来不及记录，这份“最后操作 + 内存快照”是定位关键；
 * 合并后删除，避免每次启动重复写入。
 */
async function mergeBreadcrumbs() {
  const breadcrumbPath = privateStorageDirectoryPath + '/diagnostics/breadcrumbs.log'
  try {
    if (!(await existsFile(breadcrumbPath))) return
    const content = await readFile(breadcrumbPath)
    if (content?.trim()) {
      const tail = content.length > 32768 ? content.slice(-32768) : content
      await appendFile(logPath, '\n----lx log----\n[BREADCRUMBS] 上次运行最后操作\n' + tail.trim())
    }
    await unlink(breadcrumbPath)
  } catch (err) {
    console.log('merge breadcrumbs failed:', err)
  }
}

/**
 * 把原生层崩溃日志（CrashSaver 落盘的 filesDir/crash/*.txt）并入 error.log，
 * 让“设置-错误日志”可以直接查看与导出；合并成功后删除源文件。
 */
async function mergeCrashLogs() {
  try {
    const files = await readDir(crashDirPath)
    if (!files?.length) return
    const crashFiles = files.filter(f => f.isFile && f.name.startsWith('crash-')).sort((a, b) => a.name.localeCompare(b.name))
    for (const file of crashFiles) {
      try {
        const content = await readFile(file.path)
        if (content?.length) {
          await appendFile(logPath, '\n----lx log----\n[CRASH] ' + content.trim())
        }
        await unlink(file.path)
      } catch (err) {
        console.log('merge crash log failed:', file.name, err)
      }
    }
  } catch (err) {
    // 目录不存在等，忽略
  }
}

export const init = async() => {
  return logTools.initLogFile()
}

export const getLogs = async() => {
  const content = await readFile(logPath)
  // 日志页展示与上报都只需尾部；整读数 MB 文本在 2G 设备会 OOM
  return content.length > 512 * 1024 ? content.slice(-512 * 1024) : content
}

export const clearLogs = async() => {
  return unlink(logPath).then(async() => writeFile(logPath, ''))
}

/**
 * 错误监听钩子：errorReport 通过它感知 log.error 事件（保持 log.ts → errorReport 单向依赖）。
 */
type LogErrorListener = (msg: string) => void
let logErrorListener: LogErrorListener | null = null
export const setLogErrorListener = (listener: LogErrorListener | null) => {
  logErrorListener = listener
}

export const log = {
  info(...msgs: any[]) {
    // console.info(...msgs)
    const msg = msgs.map(m => typeof m == 'string' ? m : m instanceof Error ? m.stack ?? m.message : JSON.stringify(m)).join(' ')
    if (msg.startsWith('%c')) return
    const time = new Date().toLocaleString()
    if (logTools.tempLog) {
      logTools.tempLog.push({ type: 'LOG', time, text: msg })
    } else logTools.writeLog(`${time} LOG ${msg}`)
  },
  warn(...msgs: any[]) {
    // console.warn(...msgs)
    const msg = msgs.map(m => typeof m == 'string' ? m : m instanceof Error ? m.stack ?? m.message : JSON.stringify(m)).join(' ')
    const time = new Date().toLocaleString()
    if (logTools.tempLog) {
      logTools.tempLog.push({ type: 'WARN', time, text: msg })
    } else logTools.writeLog(`${time} WARN ${msg}`)
  },
  error(...msgs: any[]) {
    const msg = msgs.map(m => typeof m == 'string' ? m : m instanceof Error ? m.stack ?? m.message : JSON.stringify(m)).join(' ')
    const time = new Date().toLocaleString()
    if (logTools.tempLog) {
      logTools.tempLog.push({ type: 'ERROR', time, text: msg })
    } else {
      logTools.writeLog(`${time} ERROR ${msg}`)
    }
    try {
      logErrorListener?.(msg)
    } catch (err) {
      console.log(err)
    }
  },
}
