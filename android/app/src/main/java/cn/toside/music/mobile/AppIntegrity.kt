package cn.toside.music.mobile

import android.app.ActivityManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Process
import android.util.Log
import java.security.MessageDigest

/**
 * 防重打包自毁（双层）：
 * 1. 本地校验：构建期（build.gradle signingCertSha256）把本次构建签名证书的 SHA-256
 *    写入 BuildConfig.SIGNING_CERT_SHA256，运行时比对当前 APK 的签名证书；不一致即自毁。
 * 2. 服务器端二次校验：应用启动后把运行时指纹上报自建服务器（integrity-verify，
 *    authorized_keys ForceCommand 限定）比对发布证书白名单，服务器明确返回
 *    DENY 且指纹与本地一致时自毁（见 src/utils/integrityCheck.ts）。
 *
 * 误伤规避（fail-open 原则）：
 * - debug 构建直接跳过（开发调试需要任意签名与调试器）；
 * - 构建期未取得指纹（如 CI 缺签名配置）或运行时读不到签名信息时放行；
 * - 服务器不可达/配置异常一律放行，仅明确 DENY 才触发；
 * - 不做调试器/root 检测：车机普遍 root 且常开 ro.debuggable，会大面积误伤。
 */
object AppIntegrity {

  private const val TAG = "AppIntegrity"

  fun check(context: Context) {
    try {
      if (BuildConfig.DEBUG) return
      val expected = BuildConfig.SIGNING_CERT_SHA256
      if (expected.isBlank()) return
      val actual = currentFingerprint(context) ?: return
      if (!actual.equals(expected, ignoreCase = true)) {
        Log.e(TAG, "signature mismatch, self-destructing")
        selfDestruct(context)
      }
    } catch (t: Throwable) {
      // 校验异常不阻塞启动
    }
  }

  /**
   * 自毁：清空应用数据（含本地缓存、用户数据与错误日志）并退出。
   * 本地签名校验与服务器端二次校验（DENY）共用此入口；debug 构建拒绝执行。
   */
  fun selfDestruct(context: Context) {
    if (BuildConfig.DEBUG) return
    try {
      // 清理由 system_server 执行，完成后系统会自行终止进程
      val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
      val cleared = am?.clearApplicationUserData() ?: false
      if (!cleared) {
        Process.killProcess(Process.myPid())
        Runtime.getRuntime().exit(0)
      }
    } catch (t: Throwable) {
      // 某些系统可能拒绝清理，兜底退出
      Process.killProcess(Process.myPid())
      Runtime.getRuntime().exit(0)
    }
  }

  /** 当前 APK 签名证书的 SHA-256（冒号分隔小写十六进制），与构建期写入格式一致 */
  fun currentFingerprint(context: Context): String? {
    return try {
      val pm = context.packageManager
      val signatures = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        pm.getPackageInfo(context.packageName, PackageManager.GET_SIGNING_CERTIFICATES)
          .signingInfo?.apkContentsSigners
      } else {
        @Suppress("DEPRECATION")
        pm.getPackageInfo(context.packageName, PackageManager.GET_SIGNATURES).signatures
      }
      val signature = signatures?.firstOrNull() ?: return null
      val digest = MessageDigest.getInstance("SHA-256").digest(signature.toByteArray())
      digest.joinToString(":") { String.format("%02x", it) }
    } catch (t: Throwable) {
      null
    }
  }
}
