import themeActions from '@/store/theme/action'
import { getTheme } from '@/theme/themes'
import { log } from '@/utils/log'
import { updateSetting } from './common'
import themeState from '@/store/theme/state'

export const setShouldUseDarkColors = (shouldUseDarkColors: boolean) => {
  themeActions.setShouldUseDarkColors(shouldUseDarkColors)
}

export const applyTheme = (theme: LX.Theme) => {
  if (!theme) return
  themeActions.setTheme(theme)
}

export const setTheme = (id: string) => {
  updateSetting({ 'theme.id': id })
  void getTheme().then(theme => {
    // 用户主题可能已损坏/被删除，getTheme 会回退到内置主题，这里再兜一层
    if (!theme || theme.id == themeState.theme.id) return
    applyTheme(theme)
  }).catch((err: Error) => {
    // 切换主题失败时保持当前主题，避免异常冒泡导致闪退
    log.error(`[theme] switch theme failed: ${err.stack ?? err.message}`)
  })
}
