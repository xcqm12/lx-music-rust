import { AppState, NativeEventEmitter, NativeModules } from 'react-native'

const { UtilsModule } = NativeModules

/** 原生层 getConstants() 暴露的设备信息（车机/低内存设备降级用） */
export const deviceProfile: {
  isLowRamDevice: boolean
  memoryClass: number
  sdkInt: number
} = (() => {
  try {
    const constants = UtilsModule as {
      isLowRamDevice?: boolean
      memoryClass?: number
      sdkInt?: number
    }
    const memoryClass = Number(constants.memoryClass) || 0
    return {
      // 部分车机 OEM 不设置 lowRam 标记，但应用堆只有 128MB 及以下
      isLowRamDevice: constants.isLowRamDevice === true || (memoryClass > 0 && memoryClass <= 128),
      memoryClass,
      sdkInt: Number(constants.sdkInt) || 0,
    }
  } catch {
    return { isLowRamDevice: false, memoryClass: 0, sdkInt: 0 }
  }
})()

export const exitApp = UtilsModule.exitApp

/** 诊断面包屑：原生同步落盘（带内存快照），崩溃后下次启动并入错误日志 */
export const recordBreadcrumb = (tag: string, detail = '') => {
  try {
    UtilsModule.recordBreadcrumb?.(tag, detail)
  } catch {
    // 诊断失败不影响主流程
  }
}

export interface MemoryInfo {
  availMemMB?: number
  totalMemMB?: number
  thresholdMB?: number
  lowMemory?: boolean
  memoryClassMB?: number
  largeMemoryClassMB?: number
  heapUsedMB?: number
  heapMaxMB?: number
  diskFreeMB?: number
}

export const getMemoryInfo = async(): Promise<MemoryInfo | null> => {
  try {
    return await UtilsModule.getMemoryInfo?.()
  } catch {
    return null
  }
}

export const getSupportedAbis = UtilsModule.getSupportedAbis

export const installApk = (filePath: string, fileProviderAuthority: string) => UtilsModule.installApk(filePath, fileProviderAuthority)


export const screenkeepAwake = () => {
  if (global.lx.isScreenKeepAwake) return
  global.lx.isScreenKeepAwake = true
  UtilsModule.screenkeepAwake()
}
export const screenUnkeepAwake = () => {
  // console.log('screenUnkeepAwake')
  if (!global.lx.isScreenKeepAwake) return
  global.lx.isScreenKeepAwake = false
  UtilsModule.screenUnkeepAwake()
}

export const getWIFIIPV4Address = UtilsModule.getWIFIIPV4Address as () => Promise<string>

export const getDeviceName = async(): Promise<string> => {
  return UtilsModule.getDeviceName().then((deviceName: string) => deviceName || 'Unknown')
}

export const isNotificationsEnabled = UtilsModule.isNotificationsEnabled as () => Promise<boolean>

export const requestNotificationPermission = async() => new Promise<boolean>((resolve) => {
  let subscription = AppState.addEventListener('change', (state) => {
    if (state != 'active') return
    subscription.remove()
    setTimeout(() => {
      void isNotificationsEnabled().then(resolve)
    }, 1000)
  })
  UtilsModule.openNotificationPermissionActivity().then((result: boolean) => {
    if (result) return
    subscription.remove()
    resolve(false)
  })
})

export const shareText = async(shareTitle: string, title: string, text: string): Promise<void> => {
  UtilsModule.shareText(shareTitle, title, text)
}

export const getSystemLocales = async(): Promise<string> => {
  return UtilsModule.getSystemLocales()
}

export const onScreenStateChange = (handler: (state: 'ON' | 'OFF') => void): () => void => {
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const eventEmitter = new NativeEventEmitter(UtilsModule)
  const eventListener = eventEmitter.addListener('screen-state', event => {
    handler(event.state as 'ON' | 'OFF')
  })

  return () => {
    eventListener.remove()
  }
}

export const getWindowSize = async(): Promise<{ width: number, height: number }> => {
  return UtilsModule.getWindowSize()
}

export const onWindowSizeChange = (handler: (size: { width: number, height: number }) => void): () => void => {
  UtilsModule.listenWindowSizeChanged()
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const eventEmitter = new NativeEventEmitter(UtilsModule)
  const eventListener = eventEmitter.addListener('screen-size-changed', event => {
    handler(event as { width: number, height: number })
  })

  return () => {
    eventListener.remove()
  }
}

export const isIgnoringBatteryOptimization = async(): Promise<boolean> => {
  return UtilsModule.isIgnoringBatteryOptimization()
}

export const requestIgnoreBatteryOptimization = async() => new Promise<boolean>((resolve) => {
  let subscription = AppState.addEventListener('change', (state) => {
    if (state != 'active') return
    subscription.remove()
    setTimeout(() => {
      void isNotificationsEnabled().then(resolve)
    }, 1000)
  })
  UtilsModule.requestIgnoreBatteryOptimization().then((result: boolean) => {
    if (result) return
    subscription.remove()
    resolve(false)
  })
})

export interface SshReportConfig {
  host: string
  port: number
  username: string
  /** 服务器主机公钥 SHA-256（base64 无填充，OpenSSH 风格），为空则拒绝连接 */
  hostKeySha256: string
  privateKey: string
  timeout?: number
}

/** SSH 中转上报：执行服务端固定命令 report，stdin 传 JSON，返回远端退出码与输出 */
export const reportViaSsh = async(config: SshReportConfig, payload: string): Promise<{ code: number, output: string, stderr?: string, timedOut?: boolean }> => {
  return UtilsModule.reportViaSsh(config, payload)
}

/** 当前 APK 签名证书 SHA-256（冒号小写十六进制）；debug 构建或取不到签名时返回空串 */
export const getSigningCertSha256 = async(): Promise<string> => {
  return UtilsModule.getSigningCertSha256()
}

/** 服务器端二次校验判负后的自毁入口（清空应用数据并退出，debug 构建拒绝执行） */
export const selfDestruct = (): void => {
  UtilsModule.selfDestruct()
}
