package cn.toside.music.mobile.utils;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.app.ActivityManager;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.graphics.Rect;
import android.net.Uri;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.util.Base64;
import android.util.Log;
import android.view.Window;
import android.view.WindowManager;

import androidx.core.app.LocaleManagerCompat;
import androidx.core.content.FileProvider;
import androidx.core.os.LocaleListCompat;

import com.facebook.react.bridge.Arguments;
import com.facebook.react.bridge.Promise;
import com.facebook.react.bridge.ReactApplicationContext;
import com.facebook.react.bridge.ReactContextBaseJavaModule;
import com.facebook.react.bridge.ReactMethod;
import com.facebook.react.bridge.ReadableMap;
import com.facebook.react.bridge.WritableArray;
import com.facebook.react.bridge.WritableMap;
import com.facebook.react.bridge.WritableNativeArray;

import com.jcraft.jsch.ChannelExec;
import com.jcraft.jsch.HostKey;
import com.jcraft.jsch.JSch;
import com.jcraft.jsch.Session;

import cn.toside.music.mobile.AppIntegrity;
import cn.toside.music.mobile.BuildConfig;
import cn.toside.music.mobile.Diagnostics;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;

public class UtilsModule extends ReactContextBaseJavaModule {
  private final ReactApplicationContext reactContext;

  private int listenerCount = 0;

  UtilsEvent utilsEvent;

  UtilsModule(ReactApplicationContext reactContext) {
    super(reactContext);
    this.reactContext = reactContext;
    utilsEvent = new UtilsEvent(reactContext);
    registerScreenBroadcastReceiver();
  }

  @Override
  public String getName() {
    return "UtilsModule";
  }

  /**
   * 同步设备信息，供 JS 在首屏渲染前决定是否启用高内存开销的效果
   * （如动态背景模糊）。车机/Android 6 老设备内存吃紧时需要降级。
   */
  @Override
  public Map<String, Object> getConstants() {
    final Map<String, Object> constants = new HashMap<>();
    boolean isLowRamDevice = false;
    int memoryClass = 0;
    try {
      ActivityManager activityManager = (ActivityManager) reactContext.getSystemService(Context.ACTIVITY_SERVICE);
      if (activityManager != null) {
        // isLowRamDevice 在 API 19+ 可用，Android 6 车机通常会返回 true
        isLowRamDevice = activityManager.isLowRamDevice();
        memoryClass = activityManager.getMemoryClass();
      }
    } catch (Exception ignored) {
      // 拿不到就按非低内存设备处理
    }
    constants.put("isLowRamDevice", isLowRamDevice);
    constants.put("memoryClass", memoryClass);
    constants.put("sdkInt", Build.VERSION.SDK_INT);
    return constants;
  }

  @ReactMethod
  public void addListener(String eventName) {
    if (listenerCount == 0) {
      // Set up any upstream listeners or background tasks as necessary
    }

    listenerCount += 1;
  }

  @ReactMethod
  public void removeListeners(Integer count) {
    listenerCount -= count;
    if (listenerCount == 0) {
      // Remove upstream listeners, stop unnecessary background tasks
    }
  }

  private void registerScreenBroadcastReceiver() {
    final IntentFilter theFilter = new IntentFilter();
    /** System Defined Broadcast */
    theFilter.addAction(Intent.ACTION_SCREEN_ON);
    theFilter.addAction(Intent.ACTION_SCREEN_OFF);

    BroadcastReceiver screenOnOffReceiver = new BroadcastReceiver() {
      @Override
      public void onReceive(Context context, Intent intent) {
        String strAction = intent.getAction();

        WritableMap params = Arguments.createMap();

        switch (Objects.requireNonNull(strAction)) {
          case Intent.ACTION_SCREEN_OFF:

            params.putString("state", "OFF");
            utilsEvent.sendEvent(utilsEvent.SCREEN_STATE, params);
            break;
          case Intent.ACTION_SCREEN_ON:
            params.putString("state", "ON");
            utilsEvent.sendEvent(utilsEvent.SCREEN_STATE, params);
            break;
        }
      }
    };

    reactContext.registerReceiver(screenOnOffReceiver, theFilter);
  }

  @ReactMethod
  public void exitApp() {
    // https://github.com/wumke/react-native-exit-app/blob/master/android/src/main/java/com/github/wumke/RNExitApp/RNExitAppModule.java
    // android.os.Process.killProcess(android.os.Process.myPid());

    // https://stackoverflow.com/questions/6330200/how-to-quit-android-application-programmatically
    Activity currentActivity = reactContext.getCurrentActivity();
    Log.d("Utils", "Exit app...");
    if (currentActivity == null) {
      Log.d("Utils", "killProcess");
      android.os.Process.killProcess(android.os.Process.myPid());
    } else {
      currentActivity.finishAndRemoveTask();
      System.exit(0);
    }
  }

  @ReactMethod
  public void getSupportedAbis(Promise promise) {
    // https://github.com/react-native-device-info/react-native-device-info/blob/ff8f672cb08fa39a887567d6e23e2f08778e8340/android/src/main/java/com/learnium/RNDeviceInfo/RNDeviceModule.java#L877
    WritableArray array = new WritableNativeArray();
    for (String abi : Build.SUPPORTED_ABIS) {
      array.pushString(abi);
    }
    promise.resolve(array);
  }

  /** 记录一条诊断面包屑（搜索/播放等关键操作 + 内存快照），供崩溃后追溯 */
  @ReactMethod
  public void recordBreadcrumb(String tag, String detail) {
    try {
      Diagnostics.INSTANCE.breadcrumb(reactContext, tag, detail == null ? "" : detail);
    } catch (Exception ignored) {
      // 诊断不应影响主流程
    }
  }

  /** 当前内存快照 { availMemMB, totalMemMB, lowMemory, heapUsedMB, heapMaxMB, ... } */
  @ReactMethod
  public void getMemoryInfo(Promise promise) {
    try {
      promise.resolve(Arguments.makeNativeMap(Diagnostics.INSTANCE.memoryInfo(reactContext)));
    } catch (Exception e) {
      promise.resolve(Arguments.createMap());
    }
  }

  @ReactMethod
  public void installApk(String filePath, String fileProviderAuthority, Promise promise) {
    // https://github.com/mikehardy/react-native-update-apk/blob/master/android/src/main/java/net/mikehardy/rnupdateapk/RNUpdateAPK.java
    File file = new File(filePath);
    if (!file.exists()) {
      Log.e("Utils", "installApk: file doe snot exist '" + filePath + "'");
      // FIXME this should take a promise and fail it
      promise.reject("Utils", "installApk: file doe snot exist '" + filePath + "'");
      return;
    }

    if (Build.VERSION.SDK_INT >= 24) {
      // API24 and up has a package installer that can handle FileProvider content:// URIs
      Uri contentUri;
      try {
        contentUri = FileProvider.getUriForFile(getReactApplicationContext(), fileProviderAuthority, file);
      } catch (Exception e) {
        // FIXME should be a Promise.reject really
        Log.e("Utils", "installApk exception with authority name '" + fileProviderAuthority + "'", e);
        promise.reject("Utils", "installApk exception with authority name '" + fileProviderAuthority + "'");
        return;
        // throw e;
      }
      Intent installApp = new Intent(Intent.ACTION_INSTALL_PACKAGE);
      installApp.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
      installApp.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
      installApp.setData(contentUri);
      installApp.putExtra(Intent.EXTRA_INSTALLER_PACKAGE_NAME, reactContext.getApplicationInfo().packageName);
      reactContext.startActivity(installApp);
      promise.resolve(null);
    } else {
      // Old APIs do not handle content:// URIs, so use an old file:// style
      String cmd = "chmod 777 " + file;
      try {
        Runtime.getRuntime().exec(cmd);
      } catch (Exception e) {
        // e.printStackTrace();
        Log.e("Utils", "installApk exception : " + e.getMessage(), e);
        promise.reject("Utils", e.getMessage());
      }
      Intent intent = new Intent(Intent.ACTION_VIEW);
      intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
      intent.setDataAndType(Uri.parse("file://" + file), "application/vnd.android.package-archive");
      reactContext.startActivity(intent);
      promise.resolve(null);
    }
  }

  @ReactMethod
  public void screenkeepAwake() {
    // https://github.com/corbt/react-native-keep-awake/blob/master/android/src/main/java/com/corbt/keepawake/KCKeepAwake.java
    final Activity activity = getCurrentActivity();

    if (activity != null) {
      activity.runOnUiThread(() -> {
        activity.getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
      });
    }
  }

  @ReactMethod
  public void screenUnkeepAwake() {
    // https://github.com/corbt/react-native-keep-awake/blob/master/android/src/main/java/com/corbt/keepawake/KCKeepAwake.java
    final Activity activity = getCurrentActivity();

    if (activity != null) {
      activity.runOnUiThread(() -> {
        activity.getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
      });
    }
  }

  /**
   Gets the device's WiFi interface IP address
   @return device's WiFi IP if connected to WiFi, else '0.0.0.0'
   */
  @ReactMethod
  public void getWIFIIPV4Address(final Promise promise) throws Exception {
    // https://github.com/pusherman/react-native-network-info/blob/master/android/src/main/java/com/pusherman/networkinfo/RNNetworkInfo.java
    WifiManager wifi = (WifiManager) reactContext.getApplicationContext().getSystemService(Context.WIFI_SERVICE);
    new Thread(new Runnable() {
      public void run() {
        try {
          WifiInfo info = wifi.getConnectionInfo();
          int ipAddress = info.getIpAddress();
          @SuppressLint("DefaultLocale") String stringip = String.format("%d.%d.%d.%d", (ipAddress & 0xff), (ipAddress >> 8 & 0xff),
            (ipAddress >> 16 & 0xff), (ipAddress >> 24 & 0xff));
          promise.resolve(stringip);
        }catch (Exception e) {
          promise.resolve(null);
        }
      }
    }).start();
  }

  // https://stackoverflow.com/a/26117646
  @ReactMethod
  public void getDeviceName(final Promise promise) {
    String manufacturer = Build.MANUFACTURER;
    String model = Build.MODEL;
    if (model.startsWith(manufacturer)) {
      promise.resolve(capitalize(model));
    } else {
      promise.resolve(capitalize(manufacturer) + " " + model);
    }
  }
  private String capitalize(String s) {
    if (s == null || s.isEmpty()) {
      return "";
    }
    char first = s.charAt(0);
    if (Character.isUpperCase(first)) {
      return s;
    } else {
      return Character.toUpperCase(first) + s.substring(1);
    }
  }

  @ReactMethod
  public void isNotificationsEnabled(final Promise promise) {
    new Thread(() -> {
      boolean enabled = NotificationPermissionUtil.isNotificationsEnabled(
        reactContext.getApplicationContext());
      promise.resolve(enabled);
    }).start();
  }

  @ReactMethod
  public void openNotificationPermissionActivity(Promise promise) {
    new Thread(() -> {
      boolean result = NotificationPermissionUtil.openNotificationPermissionActivity(
        reactContext.getApplicationContext());
      promise.resolve(result);
    }).start();
  }

  @ReactMethod
  public void shareText(String shareTitle, String title, String text) {
    Intent shareIntent = new Intent(Intent.ACTION_SEND);
    shareIntent.setType("text/plain");
    shareIntent.putExtra(Intent.EXTRA_TEXT,text);
    shareIntent.putExtra(Intent.EXTRA_SUBJECT, title);
    Objects.requireNonNull(reactContext.getCurrentActivity()).startActivity(Intent.createChooser(shareIntent, shareTitle));
  }

  // https://stackoverflow.com/questions/73463341/in-per-app-language-how-to-get-app-locale-in-api-33-if-system-locale-is-diffe
  @ReactMethod
  public void getSystemLocales(Promise promise) {
    Locale locale = null;
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      LocaleListCompat list = LocaleManagerCompat.getSystemLocales(reactContext);
      if (!list.isEmpty()) {
        locale = list.get(0);
      } else {
        promise.resolve(null);
        return;
      }
    } else {
      locale = Locale.getDefault();
    }
    if (locale == null) {
      promise.resolve("");
      return;
    }

    // 格式化成 zh_cn、en_us 等
    String language = locale.getLanguage(); // zh, en
    String country = locale.getCountry();   // CN, US
    String localeStr;

    if (!country.isEmpty()) {
      localeStr = language.toLowerCase() + "_" + country.toLowerCase();
    } else {
      localeStr = language.toLowerCase();
    }

    promise.resolve(localeStr);
  }

  // https://github.com/Anthonyzou/react-native-full-screen/blob/master/android/src/main/java/com/rn/full/screen/FullScreen.java
  //  @ReactMethod
  //  public void onFullScreen() {
  //    UiThreadUtil.runOnUiThread(() -> {
  //      Activity currentActivity = reactContext.getCurrentActivity();
  //      if (currentActivity == null) return;
  //      currentActivity.getWindow().getDecorView().setSystemUiVisibility(
  //        View.SYSTEM_UI_FLAG_LAYOUT_STABLE
  //          | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
  //          | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
  //          | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION // hide nav bar
  //          | View.SYSTEM_UI_FLAG_FULLSCREEN // hide status bar
  //          | View.SYSTEM_UI_FLAG_IMMERSIVE
  //      );
  //    });
  //  }
  //  @ReactMethod
  //  public void offFullScreen() {
  //    UiThreadUtil.runOnUiThread(() -> {
  //      Activity currentActivity = reactContext.getCurrentActivity();
  //      if (currentActivity == null) return;
  //      currentActivity.getWindow().getDecorView().setSystemUiVisibility(
  //        View.SYSTEM_UI_FLAG_LAYOUT_STABLE
  //          | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
  //          | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
  //      );
  //    });
  //  }

  @ReactMethod
  public void getWindowSize(Promise promise) {
    WritableMap params = Arguments.createMap();

    Activity currentActivity = reactContext.getCurrentActivity();
    if (currentActivity == null) {
      params.putInt("width", 0);
      params.putInt("height", 0);
      promise.resolve(params);
      return;
    }
    // 获取当前应用可用区域大小
    Window window = currentActivity.getWindow();
    Rect rect = new Rect();
    window.getDecorView().getWindowVisibleDisplayFrame(rect);
    // View decorView = window.getDecorView();
    // int width = decorView.getMeasuredWidth();
    // int height = decorView.getMeasuredHeight();
    params.putInt("width", rect.width());
    params.putInt("height", rect.height());
    promise.resolve(params);
  }

  @ReactMethod
  public void isIgnoringBatteryOptimization(Promise promise) {
    new Thread(() -> {
      boolean result = BatteryOptimizationUtil.isIgnoringBatteryOptimization(
        reactContext.getApplicationContext(),
        reactContext.getPackageName()
      );
      promise.resolve(result);
    }).start();
  }

  @ReactMethod
  public void requestIgnoreBatteryOptimization(Promise promise) {
    new Thread(() -> {
      try {
        boolean result = BatteryOptimizationUtil.requestIgnoreBatteryOptimization(
          reactContext.getApplicationContext(),
          reactContext.getPackageName()
        );
        promise.resolve(result);
      } catch (Exception e) {
        promise.reject("ERROR", e);
      }
    }).start();
  }

  /**
   * 错误日志 SSH 中转上报：以密钥认证连接中转服务器执行固定命令 report，
   * 通过 stdin 传 JSON（title/body），服务端转发到 GitHub Issues。
   * config: { host, port, username, hostKeySha256, privateKey, timeout }
   * hostKeySha256 为服务器主机公钥的 SHA-256（base64 无填充，OpenSSH 风格），
   * 连接后先比对指纹再发送数据，为空或不符一律拒绝（防 MITM）。
   * resolve: { code, output }（code 为远端命令退出码，-1 表示未取到）
   */
  @ReactMethod
  public void reportViaSsh(ReadableMap config, String payload, Promise promise) {
    new Thread(() -> {
      Session session = null;
      ChannelExec channel = null;
      try {
        String host = config.getString("host");
        int port = config.hasKey("port") ? config.getInt("port") : 22;
        String username = config.getString("username");
        String expectedFingerprint = config.getString("hostKeySha256");
        String privateKey = config.getString("privateKey");
        int timeout = config.hasKey("timeout") ? config.getInt("timeout") : 20000;
        if (host == null || username == null || privateKey == null ||
            expectedFingerprint == null || expectedFingerprint.isEmpty()) {
          promise.reject("ERR_CONFIG", "missing host/username/privateKey/hostKeySha256");
          return;
        }

        JSch jsch = new JSch();
        jsch.addIdentity("lx-report", privateKey.getBytes(StandardCharsets.UTF_8), null, null);
        session = jsch.getSession(username, host, port);
        // 先放行握手，连接成功后立即人工比对指纹，不匹配即断开
        session.setConfig("StrictHostKeyChecking", "no");
        session.setTimeout(timeout);
        session.connect(timeout);

        HostKey hostKey = session.getHostKey();
        if (hostKey == null || hostKey.getKey() == null) {
          promise.reject("ERR_HOSTKEY", "server did not present a host key");
          return;
        }
        byte[] blob = Base64.decode(hostKey.getKey(), Base64.DEFAULT);
        byte[] digest = MessageDigest.getInstance("SHA-256").digest(blob);
        String fingerprint = Base64.encodeToString(digest, Base64.NO_WRAP | Base64.NO_PADDING);
        if (!expectedFingerprint.equals(fingerprint)) {
          promise.reject("ERR_HOSTKEY_MISMATCH", "host key fingerprint mismatch (possible MITM)");
          return;
        }

        channel = (ChannelExec) session.openChannel("exec");
        channel.setCommand("report");
        channel.setInputStream(new ByteArrayInputStream(payload.getBytes(StandardCharsets.UTF_8)));
        ByteArrayOutputStream errStream = new ByteArrayOutputStream();
        channel.setErrStream(errStream);
        channel.connect(timeout);

        InputStream is = channel.getInputStream();
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] buf = new byte[4096];
        int n;
        long deadline = System.currentTimeMillis() + timeout;
        boolean readTimedOut = false;
        try {
          while ((n = is.read(buf)) >= 0) {
            out.write(buf, 0, n);
            if (out.size() > 65536) break;
            if (System.currentTimeMillis() > deadline) break;
          }
        } catch (IOException e) {
          // 读超时或连接中断：保留已收到的输出，由 JS 按输出内容判定结果
          readTimedOut = true;
        }

        // stdout 读到 EOF 时远端通道可能尚未关闭，exit status 还没同步过来。
        // 直接读会得到 -1，导致“服务端已成功建 issue”也被判为失败并触发直连兜底。
        long closeDeadline = System.currentTimeMillis() + 5000;
        while (!channel.isClosed() && System.currentTimeMillis() < closeDeadline) {
          try {
            Thread.sleep(50);
          } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            break;
          }
        }
        int code = channel.isClosed() ? channel.getExitStatus() : -1;

        String stdout = out.toString("UTF-8").trim();
        String stderr = errStream.toString("UTF-8").trim();
        WritableMap result = Arguments.createMap();
        result.putInt("code", code);
        // 正常情况下服务端把协议结果写到 stdout；异常时把 stderr 带回去便于定位
        result.putString("output", stdout.length() > 0 ? stdout : stderr);
        if (stderr.length() > 0) result.putString("stderr", stderr);
        if (readTimedOut) result.putBoolean("timedOut", true);
        promise.resolve(result);
      } catch (Exception e) {
        String message = e.getClass().getSimpleName() + ": " + (e.getMessage() == null ? "unknown error" : e.getMessage());
        promise.reject("ERR_SSH", message);
      } finally {
        if (channel != null) channel.disconnect();
        if (session != null) session.disconnect();
      }
    }).start();
  }

  /**
   * 当前 APK 签名证书 SHA-256（冒号小写十六进制，与构建期注入格式一致）。
   * debug 构建或取不到签名时返回空串，调用方据此跳过服务器端二次校验（fail-open）。
   */
  @ReactMethod
  public void getSigningCertSha256(Promise promise) {
    if (BuildConfig.DEBUG) {
      promise.resolve("");
      return;
    }
    try {
      String fingerprint = AppIntegrity.INSTANCE.currentFingerprint(getReactApplicationContext());
      promise.resolve(fingerprint == null ? "" : fingerprint);
    } catch (Exception e) {
      promise.resolve("");
    }
  }

  /** 服务器端二次校验判负后的自毁入口（清空应用数据并退出，debug 构建拒绝执行） */
  @ReactMethod
  public void selfDestruct() {
    AppIntegrity.INSTANCE.selfDestruct(getReactApplicationContext());
  }
}

