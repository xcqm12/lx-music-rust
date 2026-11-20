import { initSetting, showPactModal } from '@/core/common'
import registerPlaybackService from '@/plugins/player/service'
import initTheme from './theme'
import initI18n from './i18n'
import initUserApi from './userApi'
import initPlayer from './player'
import dataInit from './dataInit'
import initSync from './sync'
import initCommonState from './common'
import { initDeeplink } from './deeplink'
import { setApiSource } from '@/core/apiSource'
import commonActions from '@/store/common/action'
import settingState from '@/store/setting/state'
import { checkUpdate } from '@/core/version'
import { bootLog } from '@/utils/bootLog'
import { initErrorReport } from '@/utils/errorReport'
import { initIntegrityCheck } from '@/utils/integrityCheck'
import { cheatTip } from '@/utils/tools'

let isFirstPush = true
const handlePushedHomeScreen = async() => {
  await cheatTip()
  if (settingState.setting['common.isAgreePact']) {
    if (isFirstPush) {
      isFirstPush = false
      void checkUpdate()
      void initDeeplink()
    }
  } else {
    if (isFirstPush) isFirstPush = false
    showPactModal()
  }
}

let isInited = false
export default async() => {
  if (isInited) return handlePushedHomeScreen
  bootLog('Initing...')
  commonActions.setFontSize(global.lx.fontSize)
  bootLog('Font size changed.')
  const setting = await initSetting()
  bootLog('Setting inited.')

  // 主题/语言/自定义API 相互独立，车机 eMMC 上并行可明显缩短冷启动；
  // initPlayer 依赖语言资源（i18n）先就绪，仍保持 await
  const [themeDone, i18nDone, userApiDone] = await Promise.allSettled([
    initTheme(setting),
    initI18n(setting),
    initUserApi(setting),
  ])
  for (const [name, result] of [['theme', themeDone], ['i18n', i18nDone], ['user api', userApiDone]] as const) {
    if (result.status == 'rejected') {
      console.log(`init ${name} failed:`, result.reason)
      bootLog(`Init ${name} failed: ${String(result.reason)}`)
    } else {
      bootLog(`${name} inited.`)
    }
  }

  setApiSource(setting['common.apiSource'])
  bootLog('Api inited.')

  registerPlaybackService()
  bootLog('Playback Service Registered.')
  await initPlayer(setting)
  bootLog('Player inited.')
  await dataInit()
  bootLog('Data inited.')
  await initCommonState(setting)
  bootLog('Common State inited.')

  void initSync(setting)
  bootLog('Sync inited.')

  // 错误日志自动上报：需在 app.ts 的 initLog 之后（监听 log.error），
  // 启动检查内部自带延迟，不会阻塞初始化
  initErrorReport()
  bootLog('Error report inited.')

  // 防逆向·服务器端二次校验（内部自带 45s 延迟，fail-open）
  initIntegrityCheck()
  bootLog('Integrity check inited.')

  // syncSetting()

  isInited ||= true

  return handlePushedHomeScreen
}
