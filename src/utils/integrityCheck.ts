// 防逆向·服务器端二次校验：本地签名校验可被 smali 补丁摘除，这里把运行时签名
// 指纹上报到自建服务器（integrity-verify，ForceCommand 受限）比对发布证书白名单。
// 服务器明确返回 DENY 且指纹与本地一致时才自毁；超时/网络失败/服务器配置异常
// 一律放行（fail-open），白名单缺失或为空时服务器返回 ERR 而非 DENY。
import { getSigningCertSha256, reportViaSsh, selfDestruct } from '@/utils/nativeModules/utils'
import getIntegrityRelayConfig from '@/config/integrityRelay'

const CHECK_DELAY = 45000 // 启动检查延迟，避让错误上报的启动检查（30s）与初始化网络
const CHECK_TIMEOUT = 15000

const normalizeFingerprint = (text: string) => {
  return text.replace(/:/g, '').toLowerCase()
}

const runIntegrityCheck = async() => {
  try {
    const config = getIntegrityRelayConfig()
    if (!config) return
    const localFingerprint = await getSigningCertSha256()
    // debug 构建 / 构建期无签名配置 / 运行时取不到指纹 → 放行
    if (!localFingerprint) return
    const { code, output } = await reportViaSsh({
      host: config.host,
      port: config.port,
      username: config.user,
      hostKeySha256: config.hostKeySha256,
      privateKey: config.key,
      timeout: CHECK_TIMEOUT,
    }, JSON.stringify({
      certSha256: localFingerprint,
      version: process.versions?.app ?? '',
    }))
    if (code !== 0) return
    if (output.startsWith('DENY')) {
      const denied = normalizeFingerprint(output.slice(4).trim())
      // 仅当服务器否认的正是本地实测指纹时才执行自毁，防止服务器配置错误误杀
      if (denied && denied === normalizeFingerprint(localFingerprint)) {
        console.log('integrity check: server denied this build, self-destructing')
        selfDestruct()
      }
      return
    }
    console.log('integrity check:', output.slice(0, 60))
  } catch (err) {
    console.log('integrity check skipped:', (err as Error).message)
  }
}

/** 注册启动检查（每轮启动一次，结果不影响本地校验的即时判定） */
export const initIntegrityCheck = () => {
  setTimeout(() => {
    void runIntegrityCheck()
  }, CHECK_DELAY)
}
