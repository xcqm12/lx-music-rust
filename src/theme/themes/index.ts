/* eslint-disable @typescript-eslint/no-var-requires */
import { getUserTheme, saveUserTheme } from '@/utils/data'
import themes from '@/theme/themes/themes'
import settingState from '@/store/setting/state'
import themeState from '@/store/theme/state'
import { isUrl } from '@/utils'
import { privateStorageDirectoryPath } from '@/utils/fs'
import { type ImageSourcePropType } from 'react-native'

export const BG_IMAGES = {
  'jqbg.jpg': require('./images/jqbg.jpg') as ImageSourcePropType,
  'landingMoon.png': require('./images/landingMoon2.png') as ImageSourcePropType,
  'myzcbg.jpg': require('./images/myzcbg.jpg') as ImageSourcePropType,
  'xnkl.jpg': require('./images/xnkl.jpg') as ImageSourcePropType,
  '133192865_p0.jpg': require('./images/133192865_p0.jpg') as ImageSourcePropType,
  '134124384_p0.jpg': require('./images/134124384_p0.jpg') as ImageSourcePropType,
  '141419367_p0.jpg': require('./images/141419367_p0.jpg') as ImageSourcePropType,
  '144841578_p0.jpg': require('./images/144841578_p0.jpg') as ImageSourcePropType,
  '144842566_p0.jpg': require('./images/144842566_p0.jpg') as ImageSourcePropType,
} as const


let userThemes: LX.Theme[]
export const getAllThemes = async() => {
  // eslint-disable-next-line require-atomic-updates
  userThemes ??= await getUserTheme()
  return {
    themes,
    // 过滤旧版本/手工编辑导致的损坏主题，避免设置页遍历时直接抛错
    userThemes: (userThemes ?? []).filter(theme => theme?.id && theme.config?.themeColors && theme.config.extInfo),
    dataPath: privateStorageDirectoryPath + '/theme_images',
  }
}

export const saveTheme = async(theme: LX.Theme) => {
  // eslint-disable-next-line require-atomic-updates
  userThemes ??= await getUserTheme()
  const targetTheme = userThemes.find(t => t.id === theme.id)
  if (targetTheme) Object.assign(targetTheme, theme)
  else userThemes.push(theme)
  await saveUserTheme(userThemes)
}

export const removeTheme = async(id: string) => {
  // eslint-disable-next-line require-atomic-updates
  userThemes ??= await getUserTheme()
  const index = userThemes.findIndex(t => t.id === id)
  if (index < 0) return
  userThemes.splice(index, 1)
  await saveUserTheme(userThemes)
}

export type LocalTheme = typeof themes[number]
type ColorsKey = keyof LX.Theme['config']['themeColors']
const varColorRxp = /^var\((.+)\)$/
export const buildActiveThemeColors = (theme: LX.Theme): LX.ActiveTheme => {
  try {
    // 主题可能来自旧版本持久化数据或用户自定义导入，字段缺失/类型不符时不能把
    // undefined 颜色交给原生层，否则切换主题时会出现渲染崩溃。
    if (!theme?.config?.themeColors || !theme.config.extInfo) return themeState.theme
    const themeColors = theme.config.themeColors as Partial<LX.Theme['config']['themeColors']>
    const extInfo: Record<string, unknown> = { ...theme.config.extInfo }

    let bgImg: ImageSourcePropType | undefined
    const bgImage = extInfo['bg-image']
    if (typeof bgImage == 'string' && bgImage) {
      if (theme.isCustom) {
        extInfo['bg-image'] = isUrl(bgImage)
          ? bgImage
          : `${privateStorageDirectoryPath}/theme_images/${bgImage}`
      } else if (!theme.isDark || !settingState.setting['theme.hideBgDark']) {
        bgImg = BG_IMAGES[bgImage as keyof typeof BG_IMAGES]
      }
    }

    for (const [k, v] of Object.entries(extInfo)) {
      if (typeof v != 'string' || !v.startsWith('var(')) continue
      const color = themeColors[v.replace(varColorRxp, '$1') as ColorsKey]
      if (typeof color == 'string') extInfo[k] = color
    }

    const pick = (key: ColorsKey): string | undefined => {
      const value = themeColors[key]
      return typeof value == 'string' ? value : undefined
    }
    // 过滤非字符串颜色，缺失的键由上一份有效主题补齐
    const safeColors: Record<string, string> = {}
    for (const [k, v] of Object.entries(themeColors)) {
      if (typeof v == 'string') safeColors[k] = v
    }

    const activeTheme: LX.ActiveTheme = {
      ...themeState.theme,
      ...safeColors,
      ...(extInfo as unknown as LX.Theme['config']['extInfo']),
      id: theme.id,
      name: theme.name,
      isDark: theme.isDark,
      'c-font': pick('c-850') ?? themeState.theme['c-font'],
      'c-font-label': pick('c-450') ?? themeState.theme['c-font-label'],
      'c-primary-font': pick('c-primary') ?? themeState.theme['c-primary-font'],
      'c-primary-font-hover': pick('c-primary-alpha-300') ?? themeState.theme['c-primary-font-hover'],
      'c-primary-font-active': pick('c-primary-dark-100-alpha-200') ?? themeState.theme['c-primary-font-active'],
      'c-primary-background': pick('c-primary-light-400-alpha-700') ?? themeState.theme['c-primary-background'],
      'c-primary-background-hover': pick('c-primary-light-300-alpha-800') ?? themeState.theme['c-primary-background-hover'],
      'c-primary-background-active': pick('c-primary-light-100-alpha-800') ?? themeState.theme['c-primary-background-active'],
      'c-primary-input-background': pick('c-primary-light-400-alpha-700') ?? themeState.theme['c-primary-input-background'],
      'c-button-font': pick('c-primary-alpha-100') ?? themeState.theme['c-button-font'],
      'c-button-font-selected': pick('c-primary-dark-100-alpha-100') ?? themeState.theme['c-button-font-selected'],
      'c-button-background': pick('c-primary-light-400-alpha-700') ?? themeState.theme['c-button-background'],
      'c-button-background-selected': pick('c-primary-alpha-600') ?? themeState.theme['c-button-background-selected'],
      'c-button-background-hover': pick('c-primary-light-300-alpha-600') ?? themeState.theme['c-button-background-hover'],
      'c-button-background-active': pick('c-primary-light-100-alpha-600') ?? themeState.theme['c-button-background-active'],
      'c-list-header-border-bottom': pick('c-primary-alpha-900') ?? themeState.theme['c-list-header-border-bottom'],
      'c-content-background': pick('c-primary-light-1000') ?? themeState.theme['c-content-background'],
      'c-border-background': pick('c-primary-light-100-alpha-700') ?? themeState.theme['c-border-background'],
      'bg-image': bgImg,
    }
    return activeTheme
  } catch (err) {
    // 主题数据损坏时保持当前主题，避免切换主题直接闪退
    console.log('build active theme failed:', err)
    return themeState.theme
  }
}


// const copyTheme = (theme: LX.Theme): LX.Theme => {
//   return {
//     ...theme,
//     config: {
//       ...theme.config,
//       extInfo: { ...theme.config.extInfo },
//       themeColors: { ...theme.config.themeColors },
//     },
//   }
// }
// type IDS = LocalTheme['id']
export const getTheme = async() => {
  // fs.promises.readdir()
  const shouldUseDarkColors = themeState.shouldUseDarkColors
  // let themeId = settingState.setting['theme.id'] == 'auto'
  //   ? shouldUseDarkColors
  //     ? settingState.setting['theme.darkId']
  //     : settingState.setting['theme.lightId']
  //   // : 'china_ink'
  //   : settingState.setting['theme.id']
  let themeId = settingState.setting['common.isAutoTheme'] && shouldUseDarkColors
    ? 'black'
    : settingState.setting['theme.id']
  // themeId = 'naruto'
  // themeId = 'pink'
  // themeId = 'black'
  let theme: LocalTheme | LX.Theme | undefined = themes.find(theme => theme.id == themeId)
  if (!theme) {
    userThemes = await getUserTheme()
    theme = userThemes.find(theme => theme.id == themeId)
    if (!theme) {
      themeId = settingState.setting['theme.id'] == 'auto' && shouldUseDarkColors ? 'black' : 'green'
      theme = themes.find(theme => theme.id == themeId) as LX.Theme
    }
  }

  return theme
}
