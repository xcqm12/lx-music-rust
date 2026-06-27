/**
 * Rust Bridge TypeScript Module
 * Provides TypeScript interface for Rust-based JS engine
 */

import { NativeModules, Platform } from 'react-native'

const { RustBridge } = NativeModules

// Types
export interface SourceInfo {
  id: string
  name: string
  enabled: boolean
  qualitySupport: string[]
}

export interface MusicInfo {
  id: string
  name: string
  singer: string
  source: string
  albumId?: string
  albumName?: string
  duration?: string
  picUrl?: string
  lrcUrl?: string
  qualitys?: QualityInfo[]
}

export interface QualityInfo {
  quality: string
  size?: string
  url?: string
}

export interface LyricInfo {
  musicId: string
  lyric?: string
  translation?: string
  timeLine?: LyricLine[]
}

export interface LyricLine {
  time: number
  text: string
  translation?: string
}

// Rust Bridge class
class RustBridgeModule {
  private initialized = false

  async initEngine(): Promise<boolean> {
    if (this.initialized) return true
    if (Platform.OS !== 'android') {
      console.warn('RustBridge is only available on Android')
      return false
    }
    try {
      const result = await RustBridge.initEngine()
      this.initialized = result
      return result
    } catch (error) {
      console.error('Failed to init Rust engine:', error)
      return false
    }
  }

  async loadSource(sourceId: string, sourceName: string, sourceCode: string): Promise<boolean> {
    if (Platform.OS !== 'android') {
      console.warn('RustBridge is only available on Android')
      return false
    }
    try {
      return await RustBridge.loadSource(sourceId, sourceName, sourceCode)
    } catch (error) {
      console.error('Failed to load source:', error)
      throw error
    }
  }

  async search(sourceId: string, keyword: string): Promise<MusicInfo[]> {
    if (Platform.OS !== 'android') {
      console.warn('RustBridge is only available on Android')
      return []
    }
    try {
      const result = await RustBridge.search(sourceId, keyword)
      return JSON.parse(result || '[]')
    } catch (error) {
      console.error('Search error:', error)
      throw error
    }
  }

  async getMusicInfo(sourceId: string, musicId: string): Promise<MusicInfo | null> {
    if (Platform.OS !== 'android') {
      console.warn('RustBridge is only available on Android')
      return null
    }
    try {
      const result = await RustBridge.getMusicInfo(sourceId, musicId)
      return JSON.parse(result || 'null')
    } catch (error) {
      console.error('Get music info error:', error)
      throw error
    }
  }

  async getSources(): Promise<Array<[string, string]>> {
    if (Platform.OS !== 'android') {
      console.warn('RustBridge is only available on Android')
      return []
    }
    try {
      const result = await RustBridge.getSources()
      return JSON.parse(result || '[]')
    } catch (error) {
      console.error('Get sources error:', error)
      throw error
    }
  }

  async removeSource(sourceId: string): Promise<boolean> {
    if (Platform.OS !== 'android') {
      console.warn('RustBridge is only available on Android')
      return false
    }
    try {
      return await RustBridge.removeSource(sourceId)
    } catch (error) {
      console.error('Remove source error:', error)
      throw error
    }
  }

  async validateCode(code: string): Promise<boolean> {
    if (Platform.OS !== 'android') {
      console.warn('RustBridge is only available on Android')
      return false
    }
    try {
      return await RustBridge.validateCode(code)
    } catch (error) {
      console.error('Validate code error:', error)
      throw error
    }
  }
}

export const rustBridge = new RustBridgeModule()
export default rustBridge
