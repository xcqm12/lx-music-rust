import { compareVer } from '@/utils'
import { downloadNewVersion, getVersionInfo } from '@/utils/version'
import versionActions from '@/store/version/action'
import versionState, { type InitState } from '@/store/version/state'
import { getIgnoreVersion, getIgnoreVersionFailTipTime, saveIgnoreVersion, saveIgnoreVersionFailTipTime } from '@/utils/data'
import { showVersionModal } from '@/navigation'
import { Navigation } from 'react-native-navigation'
import { toast } from '@/utils/tools'

const localVersionInfo = {
  version: '1.9.1',
  desc: '同步上游 v1.9.1，新增歌曲缓存清理入口，修复多个音源问题与设置页崩溃。\n\n【新增】\n- 歌曲菜单新增「清理缓存」按钮，可用于清理错误的歌曲 URL 缓存\n\n【修复】\n- 修复打开「设置-其他-资源缓存」时应用崩溃的问题\n- 修复歌词获取失败的问题（kw/tx 歌词解压、mg 歌单解密）\n- 修复打开某些 kg 歌单时歌曲丢失的问题\n\n【优化】\n- 优化自动换源歌曲匹配机制与 tx 推荐歌单列表\n- CI 全自动构建发布：Rust 四架构库编译、APK 打包与 Release 发布\n\n【构建】\n- 恢复 SevenZeroMeowTeam 正式签名',
  history: [
    {
      version: '1.8.6',
      desc: 'React Native 文本渲染稳定性优化，修复多个潜在崩溃问题。\n\n【修复】\n- Text 组件崩溃：修复 "Text strings must be rendered within a <Text> component" 崩溃问题\n- 设置页面显示异常：修复设置页面显示 [object Object] 的问题\n- 移除自定义音源管理：从设置页面移除自定义音源管理模块\n\n【优化】\n- ESLint 规则：配置 react-native/no-raw-text 规则\n- Button 组件：优化通用 Button 组件，自动用 Text 包裹字符串类型的 children\n\n【构建】\n- 完善 ADB 端口转发配置，支持 MuMu 模拟器调试\n- 完成正式版签名打包配置，支持四架构',
    },
    {
      version: '1.8.5',
      desc: 'Rust 核心模块重大更新，修复多个编译错误并完成多架构支持。\n\n【修复】\n- AES 加密模块：重写 crypto.rs\n- JNI 生命周期：为所有 JNI FFI 函数添加生命周期注解\n- 音频缓冲区：修复 Consumer trait 相关编译错误\n\n【新增】\n- SafeText 组件：新增安全文本组件',
    },
  ],
}

export const showModal = () => {
  if (versionState.showModal) return
  versionActions.setVisibleModal(true)
  showVersionModal()
}

export const hideModal = (componentId: string) => {
  if (!versionState.showModal) return
  versionActions.setVisibleModal(false)
  void Navigation.dismissOverlay(componentId)
}

export const checkUpdate = async() => {
  versionActions.setVersionInfo({ status: 'checking' })
  let versionInfo: InitState['versionInfo'] = { ...versionState.versionInfo }
  try {
    const { version: remoteVersion, desc, history } = await getVersionInfo()
    versionInfo.newVersion = {
      version: remoteVersion,
      desc,
      history,
    }
  } catch (err) {
    versionInfo.newVersion = {
      version: '0.0.0',
      desc: '',
      history: [],
    }
  }

  if (versionInfo.newVersion.version == '0.0.0') {
    versionInfo.isUnknown = true
    versionInfo.status = 'error'
    versionInfo.newVersion = { ...localVersionInfo }
  } else {
    versionInfo.status = 'idle'
    versionInfo.isUnknown = false
    if (compareVer(versionInfo.version, versionInfo.newVersion.version) != -1) {
      versionInfo.isLatest = true
      versionInfo.newVersion = { ...localVersionInfo }
    }
  }

  versionActions.setVersionInfo(versionInfo)

  if (!versionInfo.isLatest) {
    if (versionInfo.isUnknown) {
      const time = await getIgnoreVersionFailTipTime()
      if (Date.now() - time < 7 * 86400000) return
      saveIgnoreVersionFailTipTime(Date.now())
      toast(global.i18n.t('version_tip_unknown'))
    } else if (versionInfo.newVersion.version != await getIgnoreVersion()) {
      showModal()
    }
  }
}

export const downloadUpdate = () => {
  versionActions.setVersionInfo({ status: 'downloading' })
  versionActions.setProgress({ total: 0, current: 0 })

  downloadNewVersion(versionState.versionInfo.newVersion!.version, (total: number, current: number) => {
    versionActions.setProgress({ total, current })
  }).then(() => {
    versionActions.setVersionInfo({ status: 'downloaded' })
  }).catch(() => {
    versionActions.setVersionInfo({ status: 'error' })
  })
}

export const setIgnoreVersion = (version: InitState['ignoreVersion']) => {
  versionActions.setIgnoreVersion(version)
  saveIgnoreVersion(version)
}
