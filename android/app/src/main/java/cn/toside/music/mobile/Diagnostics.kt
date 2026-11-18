package cn.toside.music.mobile

import android.app.ActivityManager
import android.app.ApplicationExitInfo
import android.content.ComponentCallbacks2
import android.content.Context
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Diagnostics - 错误/崩溃/ANR 可观测性（车机无 logcat 场景）
 *
 * 1. 面包屑：把关键操作（搜索、播放、主题切换等）与实时内存同步落盘，
 *    进程被 OOM 杀死或原生崩溃后，下次启动仍能看到“最后做了什么”。
 * 2. 主线程看门狗：主线程阻塞超过阈值时抓取全部线程栈，落盘为 crash 日志，
 *    对应 Android 6 等无法读取系统 ANR trace 的老车机。
 * 3. 退出原因：API 30+ 读取 ActivityManager 历史退出原因（ANR/原生崩溃/低内存），
 *    统一并入“设置-错误日志”。
 */
object Diagnostics {

  private const val DIAG_DIR = "diagnostics"
  private const val BREADCRUMB_FILE = "breadcrumbs.log"
  private const val MAX_BREADCRUMB_BYTES = 96 * 1024
  private const val KEEP_BREADCRUMB_BYTES = 48 * 1024
  private const val WATCHDOG_INTERVAL = 5_000L
  // 老车机冷启动/列表重建可能短暂卡顿，阈值放宽并留启动宽限期，避免误报
  private const val WATCHDOG_TIMEOUT = 12_000L
  private const val WATCHDOG_GRACE = 60_000L
  private const val WATCHDOG_MIN_GAP = 30_000L
  private const val MAX_DUMP_LENGTH = 160_000
  private const val EXIT_INFO_PREFS = "lx_diagnostics"
  private const val KEY_LAST_EXIT_TS = "last_exit_ts"

  private val lock = Any()

  @Volatile
  private var installed = false

  @Volatile
  private var lastPong = 0L

  private fun ts(): String = SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSS", Locale.US).format(Date())

  /** 安装看门狗并回收上次进程异常退出原因；须在 Application.onCreate 尽早调用 */
  @Synchronized
  fun install(context: Context) {
    if (installed) return
    installed = true
    val app = context.applicationContext
    collectExitInfo(app)
    startWatchdog(app)
    breadcrumb(app, "app.start", "version=${BuildConfig.VERSION_NAME}(${BuildConfig.VERSION_CODE})")
  }

  /** 记录一条面包屑（带内存快照），文件超过阈值时保留尾部 */
  fun breadcrumb(context: Context, tag: String, detail: String = "") {
    synchronized(lock) {
      try {
        val dir = File(context.filesDir, DIAG_DIR)
        if (!dir.exists()) dir.mkdirs()
        val file = File(dir, BREADCRUMB_FILE)
        if (file.exists() && file.length() > MAX_BREADCRUMB_BYTES) {
          val bytes = file.readBytes()
          file.writeBytes(bytes.copyOfRange(bytes.size - KEEP_BREADCRUMB_BYTES, bytes.size))
          file.appendText("--- trimmed ---\n")
        }
        val safeDetail = detail.replace('\n', ' ').take(600)
        file.appendText(
          "${ts()} $tag" +
            (if (safeDetail.isNotEmpty()) " $safeDetail" else "") +
            " | ${memorySummary(context)}\n",
        )
      } catch (t: Throwable) {
        // 面包屑失败不能影响主流程
      }
    }
  }

  /** 读取面包屑（启动时合并进 error.log 后由 JS 删除） */
  fun readBreadcrumbs(context: Context): String {
    return try {
      val file = File(File(context.filesDir, DIAG_DIR), BREADCRUMB_FILE)
      if (file.isFile) file.readText() else ""
    } catch (t: Throwable) {
      ""
    }
  }

  /** JS 可读的内存快照 */
  fun memoryInfo(context: Context): Map<String, Any> {
    val map = HashMap<String, Any>()
    try {
      val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
      if (am != null) {
        val mem = ActivityManager.MemoryInfo()
        am.getMemoryInfo(mem)
        map["availMemMB"] = (mem.availMem / 1024 / 1024).toInt()
        map["totalMemMB"] = (mem.totalMem / 1024 / 1024).toInt()
        map["thresholdMB"] = (mem.threshold / 1024 / 1024).toInt()
        map["lowMemory"] = mem.lowMemory
        map["memoryClassMB"] = am.memoryClass
        map["largeMemoryClassMB"] = am.largeMemoryClass
      }
      val runtime = Runtime.getRuntime()
      map["heapUsedMB"] = ((runtime.totalMemory() - runtime.freeMemory()) / 1024 / 1024).toInt()
      map["heapMaxMB"] = (runtime.maxMemory() / 1024 / 1024).toInt()
      map["diskFreeMB"] = (context.filesDir.usableSpace / 1024 / 1024).toInt()
    } catch (t: Throwable) {
      // 返回已采集到的字段
    }
    return map
  }

  /** onTrimMemory 回调：记录内存压力并在紧张时释放图片内存缓存 */
  fun onTrimMemory(context: Context, level: Int) {
    val name = when (level) {
      ComponentCallbacks2.TRIM_MEMORY_RUNNING_MODERATE -> "RUNNING_MODERATE"
      ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW -> "RUNNING_LOW"
      ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL -> "RUNNING_CRITICAL"
      ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN -> "UI_HIDDEN"
      ComponentCallbacks2.TRIM_MEMORY_BACKGROUND -> "BACKGROUND"
      ComponentCallbacks2.TRIM_MEMORY_MODERATE -> "MODERATE"
      ComponentCallbacks2.TRIM_MEMORY_COMPLETE -> "COMPLETE"
      else -> "LEVEL_$level"
    }
    breadcrumb(context, "memory.trim", name)
    if (level >= ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW) clearImageCache()
  }

  /** 释放 Fresco 图片内存缓存（低内存车机搜索/切歌时减少 OOM） */
  fun clearImageCache() {
    try {
      com.facebook.drawee.backends.pipeline.Fresco.getImagePipeline().clearMemoryCaches()
    } catch (t: Throwable) {
      // Fresco 未初始化或类被裁剪，忽略
    }
  }

  private fun memorySummary(context: Context): String {
    return try {
      val info = memoryInfo(context)
      val avail = info["availMemMB"] ?: "?"
      val low = info["lowMemory"] ?: "?"
      val heapUsed = info["heapUsedMB"] ?: "?"
      val heapMax = info["heapMaxMB"] ?: "?"
      val diskFree = info["diskFreeMB"] ?: "?"
      "avail=${avail}MB low=$low heap=$heapUsed/${heapMax}MB disk=${diskFree}MB"
    } catch (t: Throwable) {
      "memory=?"
    }
  }

  private fun startWatchdog(context: Context) {
    val startedAt = SystemClock.uptimeMillis()
    val worker = Runnable {
      val mainHandler = Handler(Looper.getMainLooper())
      var lastDump = 0L
      while (true) {
        lastPong = SystemClock.uptimeMillis()
        try {
          mainHandler.post { lastPong = SystemClock.uptimeMillis() }
        } catch (t: Throwable) {
          // 主线程消息队列异常，继续下一轮
        }
        try {
          Thread.sleep(WATCHDOG_INTERVAL)
        } catch (e: InterruptedException) {
          break
        }
        val now = SystemClock.uptimeMillis()
        if (now - startedAt < WATCHDOG_GRACE) continue
        val idle = now - lastPong
        if (idle >= WATCHDOG_TIMEOUT && now - lastDump >= WATCHDOG_MIN_GAP) {
          lastDump = now
          dumpAnr(context, idle)
        }
      }
    }
    Thread(worker, "lx-anr-watchdog").apply { isDaemon = true }.start()
  }

  /** 主线程长时间无响应：抓全部线程栈，作为疑似 ANR 落盘（下次启动并入错误日志） */
  private fun dumpAnr(context: Context, blockedMs: Long) {
    try {
      val sb = StringBuilder()
      sb.append("[anr] main thread blocked ").append(blockedMs).append("ms\n")
      sb.append("[threads]\n")
      val stacks = Thread.getAllStackTraces()
      for ((thread, stack) in stacks) {
        sb.append("\n\"").append(thread.name).append("\" ").append(thread.state).append('\n')
        for (element in stack) sb.append("    at ").append(element.toString()).append('\n')
        if (sb.length > MAX_DUMP_LENGTH) {
          sb.append("...truncated\n")
          break
        }
      }
      CrashSaver.saveReport(context, "疑似 ANR（主线程阻塞 ${blockedMs}ms）", sb.toString())
      breadcrumb(context, "anr.dump", "blocked=${blockedMs}ms")
    } catch (t: Throwable) {
      // 忽略
    }
  }

  /** API 30+：读取历史进程退出原因（ANR / 原生崩溃 / 低内存），增量落盘 */
  private fun collectExitInfo(context: Context) {
    if (Build.VERSION.SDK_INT < 30) return
    try {
      val am = context.getSystemService(ActivityManager::class.java) ?: return
      val reasons = am.getHistoricalProcessExitReasons(context.packageName, 0, 10)
      if (reasons.isEmpty()) return
      val prefs = context.getSharedPreferences(EXIT_INFO_PREFS, Context.MODE_PRIVATE)
      val lastTs = prefs.getLong(KEY_LAST_EXIT_TS, 0L)
      var maxTs = lastTs
      for (info in reasons) {
        if (info.timestamp <= lastTs) continue
        val reason = when (info.reason) {
          ApplicationExitInfo.REASON_ANR -> "ANR"
          ApplicationExitInfo.REASON_CRASH -> "CRASH"
          ApplicationExitInfo.REASON_CRASH_NATIVE -> "NATIVE_CRASH"
          ApplicationExitInfo.REASON_LOW_MEMORY -> "LOW_MEMORY"
          ApplicationExitInfo.REASON_EXIT_SELF -> "EXIT_SELF"
          ApplicationExitInfo.REASON_SIGNALED -> "SIGNALED"
          ApplicationExitInfo.REASON_INITIALIZATION_FAILURE -> "INIT_FAILURE"
          ApplicationExitInfo.REASON_PERMISSION_CHANGE -> "PERMISSION_CHANGE"
          ApplicationExitInfo.REASON_USER_REQUESTED -> "USER_REQUESTED"
          else -> "REASON_${info.reason}"
        }
        val detail = buildString {
          append("[exit] ").append(reason).append('\n')
          append("[time] ").append(SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.US).format(Date(info.timestamp))).append('\n')
          append("[importance] ").append(info.importance).append('\n')
          append("[rss] ").append(info.rss / 1024 / 1024).append("MB pss=").append(info.pss / 1024 / 1024).append("MB\n")
          val description = info.description
          if (!description.isNullOrEmpty()) append("[description] ").append(description).append('\n')
          try {
            info.traceInputStream?.use { input ->
              // 限读 64KB，避免超大 trace 再次占用内存
              val buffer = ByteArray(64_000)
              val read = input.read(buffer)
              if (read > 0) append("[trace]\n").append(String(buffer, 0, read, Charsets.UTF_8))
            }
          } catch (t: Throwable) {
            // trace 不可读时忽略
          }
        }
        CrashSaver.saveReport(context, "进程异常退出（$reason）", detail)
        if (info.timestamp > maxTs) maxTs = info.timestamp
      }
      prefs.edit().putLong(KEY_LAST_EXIT_TS, maxTs).apply()
    } catch (t: Throwable) {
      // 设备不支持或调用失败，忽略
    }
  }
}
