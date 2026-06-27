package com.lx.music.core

/**
 * Rust 核心桥接类
 * 用于初始化 Rust 核心和获取基本信息
 */
object RustCoreBridge {
    
    init {
        System.loadLibrary("lx_music_core")
    }
    
    /**
     * 初始化日志系统
     */
    @JvmStatic
    external fun initLogging()
    
    /**
     * 获取 Rust 核心版本
     */
    @JvmStatic
    external fun getVersion(): String
}
