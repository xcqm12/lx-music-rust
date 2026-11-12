// import { requestStoragePermission } from '@/utils/common'
import { temporaryDirectoryPath, privateStorageDirectoryPath, existsFile, appendFile, unlink, writeFile, readFile, readDir } from '@/utils/fs'

const logPath = temporaryDirectoryPath + '/error.log'
// CrashSaver.kt 写入的原生崩溃目录（filesDir/crash）
const crashDirPath = privateStorageDirectoryPath + '/crash'

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
      await mergeCrashLogs()
      if (this.tempLog?.length) this.writeLog(this.tempLog.map(m => `${m.time} ${m.type} ${m.text}`).join('\n----lx log----\n'))
      this.tempLog = null
    } catch (err) {
      console.log(err)
    }
  },
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
  return readFile(logPath)
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
/*
if (process.env.NODE_ENV !== 'development') {
  const logPath = externalDirectoryPath + '/debug.log'

  let tempLog = []

  const log = window.console.log
  const error = window.console.error
  const warn = window.console.warn

  const writeLog = msg => appendFile(logPath, '\n' + msg)

  window.console.log = (...msgs) => {
    log(...msgs)
    const msg = msgs.map(m => typeof m == 'string' ? m : JSON.stringify(m)).join(' ')
    if (msg.startsWith('%c')) return
    const time = new Date().toLocaleString()
    if (tempLog) {
      tempLog({ type: 'LOG', time, text: msg })
    } else writeLog(`${time} LOG ${msg}`)
  }
  window.console.error = (...msgs) => {
    error(...msgs)
    const msg = msgs.map(m => typeof m == 'string' ? m : JSON.stringify(m)).join(' ')
    const time = new Date().toLocaleString()
    if (tempLog) {
      tempLog({ type: 'ERROR', time, text: msg })
    } else writeLog(`${time} ERROR ${msg}`)
  }
  window.console.warn = (...msgs) => {
    warn(...msgs)
    const msg = msgs.map(m => typeof m == 'string' ? m : JSON.stringify(m)).join(' ')
    const time = new Date().toLocaleString()
    if (tempLog) {
      tempLog({ type: 'WARN', time, text: msg })
    } else writeLog(`${time} WARN ${msg}`)
  }

  const init = async() => {
    try {
      let result = await requestStoragePermission()
      if (!result) return
      let isExists = await existsFile(logPath)
      console.log(logPath, isExists)
      if (!isExists) await writeFile(logPath, '')
      writeLog(tempLog(m => `${m.time} ${m.type} ${m.text}`).join('\n'))
      tempLog = null
    } catch (err) {
      console.error(err)
    }
  }


  init()
}

 */
