package cn.toside.music.mobile;

import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import com.facebook.react.bridge.NativeModule;
import com.facebook.react.bridge.Promise;
import com.facebook.react.bridge.ReactApplicationContext;
import com.facebook.react.bridge.ReactContextBaseJavaModule;
import com.facebook.react.bridge.ReactMethod;
import com.facebook.react.modules.core.DeviceEventManagerModule;
import androidx.annotation.NonNull;
import androidx.annotation.Nullable;

import com.whl.quickjs.android.QuickJSLoader;
import com.whl.quickjs.wrapper.QuickJSContext;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Rust Bridge Module
 * Uses embedded QuickJS engine to execute custom music source JavaScript files
 */
public class RustBridgeModule extends ReactContextBaseJavaModule {
    private static final String TAG = "RustBridge";
    private static boolean isEngineInited = false;

    private final ReactApplicationContext reactContext;
    private QuickJSContext jsContext = null;
    private final Map<String, SourceContext> loadedSources = new ConcurrentHashMap<>();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    // Source context to hold JS context for each source
    private static class SourceContext {
        String id;
        String name;
        String code;
        QuickJSContext ctx;

        SourceContext(String id, String name, String code, QuickJSContext ctx) {
            this.id = id;
            this.name = name;
            this.code = code;
            this.ctx = ctx;
        }
    }

    RustBridgeModule(ReactApplicationContext context) {
        super(context);
        this.reactContext = context;
    }

    @Override
    @NonNull
    public String getName() {
        return "RustBridge";
    }

    private void initJSEngine() {
        if (jsContext == null) {
            QuickJSLoader.init();
            jsContext = QuickJSContext.create();
            setupGlobalFunctions(jsContext);
            Log.d(TAG, "QuickJS engine initialized");
        }
    }

    private void setupGlobalFunctions(QuickJSContext ctx) {
        // Setup logging function
        ctx.getGlobalObject().setProperty("__log", (args) -> {
            if (args.length > 0) {
                Log.d(TAG, "[JS] " + args[0]);
            }
            return null;
        });

        // Setup native call function
        ctx.getGlobalObject().setProperty("__nativeCall", (args) -> {
            if (args.length >= 2) {
                String action = String.valueOf(args[0]);
                String data = args.length > 1 ? String.valueOf(args[1]) : "";
                handleNativeCall(action, data);
            }
            return null;
        });

        // Setup HTTP request function (mock implementation)
        ctx.getGlobalObject().setProperty("__httpGet", (args) -> {
            String url = args.length > 0 ? String.valueOf(args[0]) : "";
            Log.d(TAG, "HTTP GET: " + url);
            // Return empty result for now
            return "{\"status\": 0, \"data\": null}";
        });

        ctx.getGlobalObject().setProperty("__httpPost", (args) -> {
            String url = args.length > 0 ? String.valueOf(args[0]) : "";
            String data = args.length > 1 ? String.valueOf(args[1]) : "";
            Log.d(TAG, "HTTP POST: " + url + " - " + data);
            return "{\"status\": 0, \"data\": null}";
        });
    }

    private void handleNativeCall(String action, String data) {
        mainHandler.post(() -> {
            sendEvent("onSourceAction", new Object[] { action, data });
        });
    }

    @ReactMethod
    public void initEngine(Promise promise) {
        try {
            if (!isEngineInited) {
                initJSEngine();
                isEngineInited = true;
                Log.d(TAG, "JS engine initialized successfully");
            }
            promise.resolve(true);
        } catch (Exception e) {
            Log.e(TAG, "Init engine error: " + e.getMessage());
            promise.reject("INIT_ERROR", e.getMessage());
        }
    }

    @ReactMethod
    public void loadSource(String sourceId, String sourceName, String sourceCode, Promise promise) {
        try {
            if (jsContext == null) {
                initJSEngine();
            }

            // Create isolated context for this source
            QuickJSContext sourceCtx = QuickJSContext.create();
            setupGlobalFunctions(sourceCtx);

            // Create module object
            String setupCode = String.format(
                    "var module = { exports: {} };\n" +
                            "var exports = module.exports;\n" +
                            "function api(url, data) { return __nativeCall(url, data); }\n" +
                            "function httpGet(url) { return __httpGet(url); }\n" +
                            "function httpPost(url, data) { return __httpPost(url, data); }\n" +
                            "function log(msg) { __log(msg); }\n" +
                            "%s\n",
                    sourceCode);

            try {
                sourceCtx.evaluate(setupCode);
                loadedSources.put(sourceId, new SourceContext(sourceId, sourceName, sourceCode, sourceCtx));
                Log.d(TAG, "Source loaded: " + sourceName + " (" + sourceId + ")");
                promise.resolve(true);
            } catch (Exception e) {
                Log.e(TAG, "JS evaluation error: " + e.getMessage());
                promise.reject("LOAD_ERROR", "JavaScript error: " + e.getMessage());
            }
        } catch (Exception e) {
            Log.e(TAG, "Load source error: " + e.getMessage());
            promise.reject("LOAD_ERROR", e.getMessage());
        }
    }

    @ReactMethod
    public void search(String sourceId, String keyword, Promise promise) {
        try {
            SourceContext source = loadedSources.get(sourceId);
            if (source == null) {
                promise.reject("SEARCH_ERROR", "Source not found: " + sourceId);
                return;
            }

            // Execute search function
            String searchCode = String.format(
                    "(function() {\n" +
                            "  var _result = [];\n" +
                            "  try {\n" +
                            "    if (typeof module.exports.search === 'function') {\n" +
                            "      var r = module.exports.search('%s');\n" +
                            "      if (r) _result = r;\n" +
                            "    }\n" +
                            "  } catch(e) { __log('Search error: ' + e.message); }\n" +
                            "  return JSON.stringify(_result);\n" +
                            "})()",
                    keyword.replace("'", "\\'"));

            try {
                Object result = source.ctx.evaluate(searchCode);
                String jsonResult = result != null ? result.toString() : "[]";
                promise.resolve(jsonResult);
            } catch (Exception e) {
                Log.e(TAG, "Search execution error: " + e.getMessage());
                promise.reject("SEARCH_ERROR", e.getMessage());
            }
        } catch (Exception e) {
            Log.e(TAG, "Search error: " + e.getMessage());
            promise.reject("SEARCH_ERROR", e.getMessage());
        }
    }

    @ReactMethod
    public void getMusicInfo(String sourceId, String musicId, Promise promise) {
        try {
            SourceContext source = loadedSources.get(sourceId);
            if (source == null) {
                promise.reject("GET_INFO_ERROR", "Source not found: " + sourceId);
                return;
            }

            String code = String.format(
                    "(function() {\n" +
                            "  try {\n" +
                            "    if (typeof module.exports.getMusicInfo === 'function') {\n" +
                            "      var r = module.exports.getMusicInfo('%s');\n" +
                            "      return r ? JSON.stringify(r) : 'null';\n" +
                            "    }\n" +
                            "  } catch(e) { __log('getMusicInfo error: ' + e.message); }\n" +
                            "  return 'null';\n" +
                            "})()",
                    musicId.replace("'", "\\'"));

            try {
                Object result = source.ctx.evaluate(code);
                String jsonResult = result != null ? result.toString() : "{}";
                promise.resolve(jsonResult);
            } catch (Exception e) {
                promise.reject("GET_INFO_ERROR", e.getMessage());
            }
        } catch (Exception e) {
            promise.reject("GET_INFO_ERROR", e.getMessage());
        }
    }

    @ReactMethod
    public void getSources(Promise promise) {
        try {
            JSONArray sources = new JSONArray();
            for (Map.Entry<String, SourceContext> entry : loadedSources.entrySet()) {
                JSONObject obj = new JSONObject();
                obj.put("id", entry.getValue().id);
                obj.put("name", entry.getValue().name);
                obj.put("enabled", true);
                sources.put(obj);
            }
            promise.resolve(sources.toString());
        } catch (Exception e) {
            promise.reject("GET_SOURCES_ERROR", e.getMessage());
        }
    }

    @ReactMethod
    public void removeSource(String sourceId, Promise promise) {
        try {
            SourceContext removed = loadedSources.remove(sourceId);
            if (removed != null) {
                removed.ctx.destroy();
                Log.d(TAG, "Source removed: " + sourceId);
                promise.resolve(true);
            } else {
                promise.resolve(false);
            }
        } catch (Exception e) {
            promise.reject("REMOVE_ERROR", e.getMessage());
        }
    }

    @ReactMethod
    public void validateCode(String code, Promise promise) {
        try {
            if (jsContext == null) {
                initJSEngine();
            }

            // Try to evaluate the code to check for syntax errors
            String testCode = "try { eval('" + code.replace("\\", "\\\\").replace("'", "\\'")
                    + "'); true; } catch(e) { false; }";
            Object result = jsContext.evaluate(testCode);
            boolean isValid = result != null && Boolean.TRUE.equals(result);
            promise.resolve(isValid);
        } catch (Exception e) {
            promise.reject("VALIDATE_ERROR", e.getMessage());
        }
    }

    private void sendEvent(String eventName, @Nullable Object params) {
        if (reactContext.hasActiveReactInstance()) {
            reactContext
                    .getJSModule(DeviceEventManagerModule.RCTDeviceEventEmitter.class)
                    .emit(eventName, params);
        }
    }
}
