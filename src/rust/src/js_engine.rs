//! JavaScript Engine Module
//! 
//! Provides JavaScript runtime using rquickjs library for executing
//! custom music source scripts.

#[cfg(feature = "js-engine")]
use rquickjs::{Context, Runtime, Module, Function};
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use log::info;
use once_cell::sync::Lazy;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JsError {
    #[error("JS runtime error: {0}")]
    RuntimeError(String),
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Engine not initialized")]
    EngineNotInitialized,
}

pub type Result<T> = std::result::Result<T, JsError>;

#[cfg(feature = "js-engine")]
/// Global QuickJS runtime instance
static JS_RUNTIME: Lazy<Arc<Mutex<Option<Runtime>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

/// Source module stored in registry
struct SourceModule {
    id: String,
    name: String,
    code: String,
}

/// JavaScript engine for executing music source scripts
#[cfg(feature = "js-engine")]
pub struct JsEngine {
    sources: HashMap<String, SourceModule>,
}

#[cfg(feature = "js-engine")]
impl JsEngine {
    /// Create a new JavaScript engine
    pub fn new() -> Result<Self> {
        info!("Initializing JS Engine with QuickJS...");

        let runtime = Runtime::new()
            .map_err(|e| JsError::RuntimeError(format!("Failed to create QuickJS runtime: {:?}", e)))?;

        *JS_RUNTIME.lock().unwrap() = Some(runtime);

        info!("JS Engine initialized successfully");

        Ok(JsEngine {
            sources: HashMap::new(),
        })
    }

    /// Load a music source script
    pub fn load_source(&mut self, id: &str, name: &str, code: &str) -> Result<()> {
        info!("Loading source: {} ({})", name, id);

        let runtime_guard = JS_RUNTIME.lock().unwrap();
        let runtime = runtime_guard.as_ref()
            .ok_or(JsError::EngineNotInitialized)?;

        // Wrap the source code with LX Music API stubs
        let wrapped_code = format!(
            r#"
            (function() {{
                var module = {{ exports: {{}} }};
                var exports = module.exports;
                
                // LX Music API stubs
                var lx = {{
                    request: function(options) {{
                        return {{ statusCode: 200, body: '', error: null }};
                    }},
                    crypto: {{
                        md5: function(s) {{ return ''; }},
                        sha256: function(s) {{ return ''; }},
                        aesEncrypt: function(data, key) {{ return ''; }},
                        aesDecrypt: function(data, key) {{ return ''; }},
                        rsaEncrypt: function(data, key) {{ return ''; }}
                    }},
                    base64: {{
                        encode: function(s) {{ return ''; }},
                        decode: function(s) {{ return ''; }}
                    }},
                    buffer: {{
                        from: function(s) {{ return new ArrayBuffer(s.length); }}
                    }},
                    version: '1.0.0'
                }};
                
                // Execute source code
                try {{
                    {code}
                }} catch(e) {{
                    console.log('Source load error:', e);
                }}
                
                // Store module for later access
                globalThis['__source_' + '{id}'] = module;
            }})();
            "#,
            code = code,
            id = id
        );

        let ctx = Context::full(runtime)
            .map_err(|e| JsError::RuntimeError(format!("Failed to create context: {:?}", e)))?;

        ctx.with(|ctx| {
            ctx.eval::<(), _>(&wrapped_code)
                .map_err(|e| JsError::ExecutionError(format!("Script error: {:?}", e)))?;
            Ok(())
        })?;

        self.sources.insert(id.to_string(), SourceModule {
            id: id.to_string(),
            name: name.to_string(),
            code: code.to_string(),
        });

        info!("Source loaded successfully: {}", id);
        Ok(())
    }

    /// Search music on a source
    pub fn search(&self, source_id: &str, keyword: &str) -> Result<Vec<serde_json::Value>> {
        info!("Searching on source {}: {}", source_id, keyword);

        if !self.sources.contains_key(source_id) {
            return Err(JsError::ModuleNotFound(source_id.to_string()));
        }

        let runtime_guard = JS_RUNTIME.lock().unwrap();
        let runtime = runtime_guard.as_ref()
            .ok_or(JsError::EngineNotInitialized)?;

        let search_code = format!(
            r#"
            (function() {{
                var module = globalThis['__source_' + '{source_id}'];
                if (!module) return '[]';
                try {{
                    if (typeof module.exports.search === 'function') {{
                        return JSON.stringify(module.exports.search('{keyword}'));
                    }}
                    return '[]';
                }} catch(e) {{
                    console.log('Search error:', e);
                    return '[]';
                }}
            }})();
            "#,
            source_id = source_id,
            keyword = keyword.replace('\'', "\\'")
        );

        let ctx = Context::full(runtime)
            .map_err(|e| JsError::RuntimeError(format!("Failed to create context: {:?}", e)))?;

        let result: String = ctx.with(|ctx| {
            ctx.eval::<String, _>(&search_code)
                .map_err(|e| JsError::ExecutionError(format!("Search failed: {:?}", e)))?
        });

        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result)
            .map_err(|e| JsError::SerializationError(e.to_string()))?;

        Ok(parsed)
    }

    /// Get music URL from source
    pub fn get_music_url(&self, source_id: &str, music_id: &str, quality: &str) -> Result<serde_json::Value> {
        info!("Getting music URL for {} quality: {}", music_id, quality);

        if !self.sources.contains_key(source_id) {
            return Err(JsError::ModuleNotFound(source_id.to_string()));
        }

        let runtime_guard = JS_RUNTIME.lock().unwrap();
        let runtime = runtime_guard.as_ref()
            .ok_or(JsError::EngineNotInitialized)?;

        let code = format!(
            r#"
            (function() {{
                var module = globalThis['__source_' + '{source_id}'];
                if (!module) return 'null';
                try {{
                    if (typeof module.exports.getUrl === 'function') {{
                        return JSON.stringify(module.exports.getUrl('{music_id}', '{quality}'));
                    }} else if (typeof module.exports.songUrl === 'function') {{
                        return JSON.stringify(module.exports.songUrl({{ id: '{music_id}' }}, '{quality}'));
                    }}
                    return 'null';
                }} catch(e) {{
                    console.log('getMusicUrl error:', e);
                    return 'null';
                }}
            }})();
            "#,
            source_id = source_id,
            music_id = music_id.replace('\'', "\\'"),
            quality = quality.replace('\'', "\\'")
        );

        let ctx = Context::full(runtime)
            .map_err(|e| JsError::RuntimeError(format!("Failed to create context: {:?}", e)))?;

        let result: String = ctx.with(|ctx| {
            ctx.eval::<String, _>(&code)
                .map_err(|e| JsError::ExecutionError(format!("getUrl failed: {:?}", e)))?
        });

        let parsed: serde_json::Value = serde_json::from_str(&result)
            .map_err(|e| JsError::SerializationError(e.to_string()))?;

        Ok(parsed)
    }

    /// Get lyric from source
    pub fn get_lyric(&self, source_id: &str, music_id: &str) -> Result<serde_json::Value> {
        if !self.sources.contains_key(source_id) {
            return Err(JsError::ModuleNotFound(source_id.to_string()));
        }

        let runtime_guard = JS_RUNTIME.lock().unwrap();
        let runtime = runtime_guard.as_ref()
            .ok_or(JsError::EngineNotInitialized)?;

        let code = format!(
            r#"
            (function() {{
                var module = globalThis['__source_' + '{source_id}'];
                if (!module) return 'null';
                try {{
                    if (typeof module.exports.getLyric === 'function') {{
                        return JSON.stringify(module.exports.getLyric('{music_id}'));
                    }} else if (typeof module.exports.lyric === 'function') {{
                        return JSON.stringify(module.exports.lyric({{ id: '{music_id}' }}));
                    }}
                    return 'null';
                }} catch(e) {{
                    console.log('getLyric error:', e);
                    return 'null';
                }}
            }})();
            "#,
            source_id = source_id,
            music_id = music_id.replace('\'', "\\'")
        );

        let ctx = Context::full(runtime)
            .map_err(|e| JsError::RuntimeError(format!("Failed to create context: {:?}", e)))?;

        let result: String = ctx.with(|ctx| {
            ctx.eval::<String, _>(&code)
                .map_err(|e| JsError::ExecutionError(format!("getLyric failed: {:?}", e)))?
        });

        let parsed: serde_json::Value = serde_json::from_str(&result)
            .map_err(|e| JsError::SerializationError(e.to_string()))?;

        Ok(parsed)
    }

    /// Get album artwork from source
    pub fn get_pic(&self, source_id: &str, music_id: &str) -> Result<serde_json::Value> {
        if !self.sources.contains_key(source_id) {
            return Err(JsError::ModuleNotFound(source_id.to_string()));
        }

        let runtime_guard = JS_RUNTIME.lock().unwrap();
        let runtime = runtime_guard.as_ref()
            .ok_or(JsError::EngineNotInitialized)?;

        let code = format!(
            r#"
            (function() {{
                var module = globalThis['__source_' + '{source_id}'];
                if (!module) return 'null';
                try {{
                    if (typeof module.exports.getPic === 'function') {{
                        return JSON.stringify(module.exports.getPic('{music_id}'));
                    }} else if (typeof module.exports.pic === 'function') {{
                        return JSON.stringify(module.exports.pic({{ id: '{music_id}' }}));
                    }}
                    return 'null';
                }} catch(e) {{
                    console.log('getPic error:', e);
                    return 'null';
                }}
            }})();
            "#,
            source_id = source_id,
            music_id = music_id.replace('\'', "\\'")
        );

        let ctx = Context::full(runtime)
            .map_err(|e| JsError::RuntimeError(format!("Failed to create context: {:?}", e)))?;

        let result: String = ctx.with(|ctx| {
            ctx.eval::<String, _>(&code)
                .map_err(|e| JsError::ExecutionError(format!("getPic failed: {:?}", e)))?
        });

        let parsed: serde_json::Value = serde_json::from_str(&result)
            .map_err(|e| JsError::SerializationError(e.to_string()))?;

        Ok(parsed)
    }

    /// Get all loaded sources
    pub fn get_sources(&self) -> Vec<(String, String)> {
        self.sources.iter()
            .map(|(id, source)| (id.clone(), source.name.clone()))
            .collect()
    }

    /// Remove a source
    pub fn remove_source(&mut self, source_id: &str) -> bool {
        info!("Removing source: {}", source_id);
        self.sources.remove(source_id).is_some()
    }

    /// Validate source code
    pub fn validate_code(code: &str) -> Result<()> {
        let runtime = Runtime::new()
            .map_err(|e| JsError::RuntimeError(format!("Failed to create runtime: {:?}", e)))?;

        let ctx = Context::full(&runtime)
            .map_err(|e| JsError::RuntimeError(format!("Failed to create context: {:?}", e)))?;

        ctx.with(|ctx| {
            ctx.eval::<(), _>(code)
                .map_err(|e| JsError::ExecutionError(format!("Validation failed: {:?}", e)))?;
            Ok(())
        })?;

        Ok(())
    }

    /// Execute arbitrary JavaScript code
    pub fn execute(&self, code: &str) -> Result<String> {
        let runtime_guard = JS_RUNTIME.lock().unwrap();
        let runtime = runtime_guard.as_ref()
            .ok_or(JsError::EngineNotInitialized)?;

        let ctx = Context::full(runtime)
            .map_err(|e| JsError::RuntimeError(format!("Failed to create context: {:?}", e)))?;

        let result: String = ctx.with(|ctx| {
            ctx.eval::<String, _>(code)
                .map_err(|e| JsError::ExecutionError(format!("Execution failed: {:?}", e)))?
        });

        Ok(result)
    }
}

#[cfg(feature = "js-engine")]
impl Default for JsEngine {
    fn default() -> Self {
        Self::new().expect("Failed to initialize default JsEngine")
    }
}

// Stub implementation when JS engine is not enabled
#[cfg(not(feature = "js-engine"))]
pub struct JsEngine {
    sources: HashMap<String, SourceModule>,
}

#[cfg(not(feature = "js-engine"))]
impl JsEngine {
    pub fn new() -> Result<Self> {
        info!("JS Engine disabled, using stub implementation");
        Ok(JsEngine {
            sources: HashMap::new(),
        })
    }

    pub fn load_source(&mut self, id: &str, name: &str, code: &str) -> Result<()> {
        info!("Stub: Loading source {} ({})", name, id);
        self.sources.insert(id.to_string(), SourceModule {
            id: id.to_string(),
            name: name.to_string(),
            code: code.to_string(),
        });
        Ok(())
    }

    pub fn search(&self, source_id: &str, keyword: &str) -> Result<Vec<serde_json::Value>> {
        Ok(vec![])
    }

    pub fn get_music_url(&self, source_id: &str, music_id: &str, quality: &str) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }

    pub fn get_lyric(&self, source_id: &str, music_id: &str) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }

    pub fn get_pic(&self, source_id: &str, music_id: &str) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }

    pub fn get_sources(&self) -> Vec<(String, String)> {
        self.sources.iter()
            .map(|(id, source)| (id.clone(), source.name.clone()))
            .collect()
    }

    pub fn remove_source(&mut self, source_id: &str) -> bool {
        self.sources.remove(source_id).is_some()
    }

    pub fn validate_code(_code: &str) -> Result<()> {
        Ok(())
    }

    pub fn execute(&self, _code: &str) -> Result<String> {
        Ok(String::new())
    }
}

#[cfg(not(feature = "js-engine"))]
impl Default for JsEngine {
    fn default() -> Self {
        Self::new().expect("Failed to initialize stub JsEngine")
    }
}