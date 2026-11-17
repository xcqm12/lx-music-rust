import { buildActiveThemeColors } from '@/theme/themes'
import { log } from '@/utils/log'
import state from './state'


export default {
  setTheme(theme: LX.Theme) {
    try {
      const activeTheme = buildActiveThemeColors(theme)
      // 极端情况下主题数据损坏，保持当前主题而不是把无效对象交给所有页面重渲染
      if (!activeTheme?.id) return
      state.theme = activeTheme
    } catch (err) {
      log.error(`[theme] apply theme failed: ${(err as Error).stack ?? (err as Error).message}`)
      return
    }
    // ThemeContext.displayName
    global.state_event.themeUpdated(state.theme)
  },
  setShouldUseDarkColors(shouldUseDarkColors: boolean) {
    if (state.shouldUseDarkColors == shouldUseDarkColors) return
    state.shouldUseDarkColors = shouldUseDarkColors
  },
}
