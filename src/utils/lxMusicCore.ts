/**
 * LX Music Core - React Native TurboModule
 * 
 * TypeScript wrapper for the Rust core library via TurboModule.
 * Provides a clean API for UI components to interact with native code.
 * 
 * Architecture: JS -> TurboModule (TS) -> C++ -> JNI -> Rust .so
 */

import { NativeModules, Platform } from 'react-native';

// ============================================================================
// Type Definitions
// ============================================================================

export interface MusicInfo {
  id: string;
  name: string;
  singer: string;
  source: string;
  albumId?: string;
  albumName?: string;
  duration?: string;
  picUrl?: string;
  lrcUrl?: string;
  qualitys?: QualityInfo[];
  url?: string;
}

export interface QualityInfo {
  quality: string;
  size?: string;
  url?: string;
}

export interface LyricInfo {
  lrc?: string;
  lrcT?: string;
  lrcRoma?: string;
  trc?: string;
}

export interface LyricLine {
  time: number;
  text: string;
  translation?: string;
}

export interface ProgressInfo {
  currentTime: number;
  duration: number;
}

export interface PlayerState {
  isPlaying: boolean;
  isPaused: boolean;
  currentMusic: MusicInfo | null;
  playlist: MusicInfo[];
  currentIndex: number;
  playMode: PlayMode;
  progress: ProgressInfo;
  volume: number;
  playbackRate: number;
  playedList: string[];
}

export type PlayMode = 'list_loop' | 'random' | 'list' | 'single_loop';

export interface SourceInfo {
  id: string;
  name: string;
  enabled: boolean;
}

// ============================================================================
// Native Module Interface
// ============================================================================

interface LXMusicCoreModule {
  // Initialization
  initEngine(): Promise<boolean>;
  isInitialized(): Promise<boolean>;
  
  // Music source functions
  loadSource(sourceId: string, sourceName: string, sourceCode: string): Promise<string>;
  searchMusic(sourceId: string, keyword: string): Promise<string>;
  getMusicUrl(sourceId: string, musicId: string, quality: string): Promise<string>;
  getLyric(sourceId: string, musicId: string): Promise<string>;
  getPic(sourceId: string, musicId: string): Promise<string>;
  getSources(): Promise<string[]>;
  
  // Player functions
  playerPlay(): void;
  playerPause(): void;
  playerStop(): void;
  playerToggle(): void;
  playerNext(): void;
  playerPrev(): void;
  playerSeek(timeMs: number): void;
  playerSetVolume(volume: number): void;
  playerSetPlaybackRate(rate: number): void;
  playerSetPlayMode(mode: number): void;
  playerSetPlaylist(playlistJson: string): Promise<string>;
  playerPlayAtIndex(index: number): void;
  playerGetState(): Promise<string>;
  playerAddToPlaylist(musicJson: string): Promise<string>;
  playerRemoveFromPlaylist(index: number): void;
  playerClearPlaylist(): void;
  
  // Lyric functions
  lyricSetLyric(lyric: string, translation: string): void;
  lyricGetCurrentLine(timeMs: number): Promise<string>;
  lyricGetLineIndex(timeMs: number): Promise<number>;
  lyricGetLines(): Promise<string>;
  lyricSetPlaybackRate(rate: number): void;
  lyricToggleTranslation(show: boolean): void;
  lyricIsShowTranslation(): Promise<boolean>;
  lyricClear(): void;
}

// ============================================================================
// Native Module Registration
// ============================================================================

const NATIVE_MODULE_NAME = 'LXMusicCore';

let NativeLXMusicCore: LXMusicCoreModule | undefined;

try {
  if (Platform.OS === 'android') {
    NativeLXMusicCore = NativeModules[NATIVE_MODULE_NAME];
  }
} catch (error) {
  console.warn(`Native module ${NATIVE_MODULE_NAME} not available:`, error);
}

// ============================================================================
// LXMusicCore API
// ============================================================================

class LXMusicCoreAPI {
  private initialized = false;

  /**
   * Initialize the native engine
   */
  async init(): Promise<boolean> {
    if (this.initialized) return true;
    
    if (!NativeLXMusicCore) {
      console.warn('LXMusicCore native module not available');
      return false;
    }

    try {
      const result = await NativeLXMusicCore.initEngine();
      this.initialized = result;
      return result;
    } catch (error) {
      console.error('Failed to initialize LXMusicCore:', error);
      return false;
    }
  }

  /**
   * Check if native engine is initialized
   */
  async isInitialized(): Promise<boolean> {
    if (!NativeLXMusicCore) return false;
    
    try {
      return await NativeLXMusicCore.isInitialized();
    } catch (error) {
      console.error('Failed to check initialization:', error);
      return false;
    }
  }

  // ========================================================================
  // Music Source Functions
  // ========================================================================

  /**
   * Load a music source script
   */
  async loadSource(
    sourceId: string,
    sourceName: string,
    sourceCode: string
  ): Promise<{ success: boolean; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.loadSource(sourceId, sourceName, sourceCode);
      return { success: result === 'OK', error: result !== 'OK' ? result : undefined };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  /**
   * Search music on a source
   */
  async searchMusic(
    sourceId: string,
    keyword: string
  ): Promise<{ success: boolean; data?: MusicInfo[]; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.searchMusic(sourceId, keyword);
      const data = JSON.parse(result) as MusicInfo[];
      return { success: true, data };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  /**
   * Get music URL from source
   */
  async getMusicUrl(
    sourceId: string,
    musicId: string,
    quality: string = 'high'
  ): Promise<{ success: boolean; url?: string; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.getMusicUrl(sourceId, musicId, quality);
      const parsed = JSON.parse(result);
      return { success: true, url: parsed?.url };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  /**
   * Get lyric from source
   */
  async getLyric(
    sourceId: string,
    musicId: string
  ): Promise<{ success: boolean; lyric?: LyricInfo; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.getLyric(sourceId, musicId);
      const lyric = JSON.parse(result) as LyricInfo;
      return { success: true, lyric };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  /**
   * Get all loaded sources
   */
  async getSources(): Promise<{ success: boolean; sources?: SourceInfo[]; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.getSources();
      const sources: SourceInfo[] = [];
      for (let i = 0; i < result.length; i += 2) {
        sources.push({
          id: result[i],
          name: result[i + 1],
          enabled: true,
        });
      }
      return { success: true, sources };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  // ========================================================================
  // Player Functions
  // ========================================================================

  playerPlay(): void {
    NativeLXMusicCore?.playerPlay();
  }

  playerPause(): void {
    NativeLXMusicCore?.playerPause();
  }

  playerStop(): void {
    NativeLXMusicCore?.playerStop();
  }

  playerToggle(): void {
    NativeLXMusicCore?.playerToggle();
  }

  playerNext(): void {
    NativeLXMusicCore?.playerNext();
  }

  playerPrev(): void {
    NativeLXMusicCore?.playerPrev();
  }

  playerSeek(timeMs: number): void {
    NativeLXMusicCore?.playerSeek(timeMs);
  }

  playerSetVolume(volume: number): void {
    NativeLXMusicCore?.playerSetVolume(volume);
  }

  playerSetPlaybackRate(rate: number): void {
    NativeLXMusicCore?.playerSetPlaybackRate(rate);
  }

  playerSetPlayMode(mode: PlayMode | number): void {
    const modeMap: Record<PlayMode, number> = {
      list_loop: 0,
      random: 1,
      list: 2,
      single_loop: 3,
    };
    const modeValue = typeof mode === 'number' ? mode : modeMap[mode];
    NativeLXMusicCore?.playerSetPlayMode(modeValue);
  }

  async playerSetPlaylist(
    playlist: MusicInfo[]
  ): Promise<{ success: boolean; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.playerSetPlaylist(JSON.stringify(playlist));
      return { success: result === 'OK', error: result !== 'OK' ? result : undefined };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  playerPlayAtIndex(index: number): void {
    NativeLXMusicCore?.playerPlayAtIndex(index);
  }

  async playerGetState(): Promise<{ success: boolean; state?: PlayerState; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.playerGetState();
      const state = JSON.parse(result) as PlayerState;
      return { success: true, state };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  async playerAddToPlaylist(
    music: MusicInfo
  ): Promise<{ success: boolean; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.playerAddToPlaylist(JSON.stringify(music));
      return { success: result === 'OK', error: result !== 'OK' ? result : undefined };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  playerRemoveFromPlaylist(index: number): void {
    NativeLXMusicCore?.playerRemoveFromPlaylist(index);
  }

  playerClearPlaylist(): void {
    NativeLXMusicCore?.playerClearPlaylist();
  }

  // ========================================================================
  // Lyric Functions
  // ========================================================================

  lyricSetLyric(lyric: string, translation: string = ''): void {
    NativeLXMusicCore?.lyricSetLyric(lyric, translation);
  }

  async lyricGetCurrentLine(
    timeMs: number
  ): Promise<{ success: boolean; line?: LyricLine; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.lyricGetCurrentLine(timeMs);
      const line = result === 'null' ? undefined : JSON.parse(result) as LyricLine;
      return { success: true, line };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  async lyricGetLineIndex(timeMs: number): Promise<number> {
    if (!NativeLXMusicCore) return -1;
    
    try {
      return await NativeLXMusicCore.lyricGetLineIndex(timeMs);
    } catch (error) {
      return -1;
    }
  }

  async lyricGetLines(): Promise<{ success: boolean; lines?: LyricLine[]; error?: string }> {
    if (!NativeLXMusicCore) {
      return { success: false, error: 'Native module not available' };
    }

    try {
      const result = await NativeLXMusicCore.lyricGetLines();
      const lines = JSON.parse(result) as LyricLine[];
      return { success: true, lines };
    } catch (error) {
      return { success: false, error: String(error) };
    }
  }

  lyricSetPlaybackRate(rate: number): void {
    NativeLXMusicCore?.lyricSetPlaybackRate(rate);
  }

  lyricToggleTranslation(show: boolean): void {
    NativeLXMusicCore?.lyricToggleTranslation(show);
  }

  async lyricIsShowTranslation(): Promise<boolean> {
    if (!NativeLXMusicCore) return false;
    
    try {
      return await NativeLXMusicCore.lyricIsShowTranslation();
    } catch (error) {
      return false;
    }
  }

  lyricClear(): void {
    NativeLXMusicCore?.lyricClear();
  }
}

// Export singleton instance
export const LXMusicCore = new LXMusicCoreAPI();

// Export types
export type { LXMusicCoreModule, PlayerState, MusicInfo, LyricLine, LyricInfo, SourceInfo, PlayMode };