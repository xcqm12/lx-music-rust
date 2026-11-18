package cn.toside.music.mobile

import android.app.ActivityManager
import android.content.Context
import android.os.Build
import android.os.Process
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * CrashSaver - 同步落盘 Java 层未捕获异常
 *
 * 在任何 RN/JS 基础设施就绪之前安装，保证崩溃信息能可靠写到应用私有目录，
 * 供下次启动时并入“设置-错误日志”，解决车机等无法抓 logcat 的场景。
 */
object CrashSaver {

  private const val CRASH_DIR = "crash"
  private const val MAX_CRASH_FILES = 5
  private const val MAX_STACK_LENGTH = 60_000
  private const val MAX_REPORT_LENGTH = 160_000

  @Volatile
  private var installed = false

  /**
   * 安装默认未捕获异常处理器。必须在 MainApplication.onCreate 最先调用。
   * 链式转发给先前的处理器，不破坏 react-native-exception-handler 等已有逻辑。
   */
  @Synchronized
  fun install(context: Context) {
    if (installed) return
    installed = true
    val previous = Thread.getDefaultUncaughtExceptionHandler()
    Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
      try {
        save(context, thread, throwable)
      } catch (t: Throwable) {
        // 落盘失败也不能吞掉原始崩溃
      }
      previous?.uncaughtException(thread, throwable)
    }
  }

  /** 供外部（如启动时）主动记录一条非致命异常信息 */
  fun saveReport(context: Context, title: String, body: String) {
    try {
      val dir = File(context.filesDir, CRASH_DIR)
      if (!dir.exists()) dir.mkdirs()
      val time = System.currentTimeMillis()
      val name = "crash-" + SimpleDateFormat("yyyyMMdd-HHmmss-SSS", Locale.US).format(Date(time)) + ".txt"
      val file = File(dir, name)
      val safeBody = if (body.length > MAX_REPORT_LENGTH) body.substring(0, MAX_REPORT_LENGTH) + "\n...truncated" else body
      file.writeText(buildHeader(context) + "\n[report] $title\n$safeBody\n")
      trimOldFiles(dir)
    } catch (t: Throwable) {
      // ignore
    }
  }

  /** 读取全部崩溃日志（启动时并入 error.log 后可清掉已合并的文件） */
  fun readCrashFiles(context: Context): List<File> {
    return try {
      val dir = File(context.filesDir, CRASH_DIR)
      if (dir.isDirectory) dir.listFiles { f -> f.isFile && f.name.startsWith("crash-") }?.sortedBy { it.name } ?: emptyList()
      else emptyList()
    } catch (t: Throwable) {
      emptyList()
    }
  }

  /** 清除已并入错误日志的崩溃文件 */
  fun clearCrashFiles(context: Context) {
    try {
      readCrashFiles(context).forEach { it.delete() }
    } catch (t: Throwable) {
      // ignore
    }
  }

  private fun save(context: Context, thread: Thread, throwable: Throwable) {
    val dir = File(context.filesDir, CRASH_DIR)
    if (!dir.exists()) dir.mkdirs()
    val time = System.currentTimeMillis()
    val name = "crash-" + SimpleDateFormat("yyyyMMdd-HHmmss", Locale.US).format(Date(time)) + ".txt"
    val file = File(dir, name)
    val stack = throwable.stackTraceToString().let {
      if (it.length > MAX_STACK_LENGTH) it.substring(0, MAX_STACK_LENGTH) + "\n...truncated" else it
    }
    file.writeText(buildHeader(context) + "\n[thread] " + thread.name + "\n[stack]\n" + stack + "\n")
    trimOldFiles(dir)
  }

  private fun buildHeader(context: Context): String {
    val sb = StringBuilder()
    sb.append("[time] ").append(SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.US).format(Date())).append('\n')
    try {
      sb.append("[version] ").append(BuildConfig.VERSION_NAME).append('(').append(BuildConfig.VERSION_CODE).append(")\n")
    } catch (t: Throwable) { /* ignore */ }
    sb.append("[device] ").append(Build.MANUFACTURER).append(' ').append(Build.MODEL).append('\n')
    sb.append("[system] Android ").append(Build.VERSION.RELEASE).append(" (API ").append(Build.VERSION.SDK_INT).append(")\n")
    try {
      sb.append("[abi] ").append(Build.SUPPORTED_ABIS.joinToString(",")).append('\n')
    } catch (t: Throwable) { /* ignore */ }
    try {
      val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
      if (am != null) {
        val mem = ActivityManager.MemoryInfo()
        am.getMemoryInfo(mem)
        sb.append("[memory] avail=").append(mem.availMem / 1024 / 1024).append("MB total=")
          .append(mem.totalMem / 1024 / 1024).append("MB lowMemory=").append(mem.lowMemory)
          .append(" threshold=").append(mem.threshold / 1024 / 1024).append("MB\n")
        sb.append("[heap] max=").append(am.memoryClass).append("MB largeHeap=").append(am.largeMemoryClass).append("MB\n")
      }
    } catch (t: Throwable) { /* ignore */ }
    try {
      sb.append("[pid] ").append(Process.myPid()).append('\n')
    } catch (t: Throwable) { /* ignore */ }
    return sb.toString()
  }

  private fun trimOldFiles(dir: File) {
    try {
      val files = dir.listFiles { f -> f.isFile && f.name.startsWith("crash-") } ?: return
      if (files.size > MAX_CRASH_FILES) {
        files.sortedBy { it.name }.take(files.size - MAX_CRASH_FILES).forEach { it.delete() }
      }
    } catch (t: Throwable) { /* ignore */ }
  }
}
