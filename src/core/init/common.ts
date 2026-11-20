import playerState from '@/store/player/state'
import { prefetch } from '@/components/common/ImageBackground'
import { setBgPic } from '@/core/common'

const formatUri = <T extends string | null>(url: T) => {
  return (typeof url == 'string' && url.startsWith('/')) ? `file://${url}` : url
}

export default async(setting: LX.AppSetting) => {
  let pic = playerState.musicInfo.pic
  let isDynamicBg = setting['theme.dynamicBg']
  const handleUpdatePic = (pic: string) => {
    if (!pic) return
    const picUrl = formatUri(pic)
    void prefetch(picUrl).then(() => {
      if (pic != playerState.musicInfo.pic || !isDynamicBg) return
      setBgPic(picUrl)
    })
  }
  const handlePicUpdate = () => {
    if (playerState.musicInfo.pic && playerState.musicInfo.pic != playerState.loadErrorPicUrl) {
      pic = playerState.musicInfo.pic
      if (!isDynamicBg) return
      handleUpdatePic(pic)
    }
  }
  const handleConfigUpdate = (keys: Array<keyof LX.AppSetting>, setting: Partial<LX.AppSetting>) => {
    if (!keys.includes('theme.dynamicBg')) return
    isDynamicBg = setting['theme.dynamicBg']!
    if (isDynamicBg) {
      if (pic) handleUpdatePic(pic)
    } else setBgPic(null)
  }
  handlePicUpdate()
  global.state_event.on('playerMusicInfoChanged', handlePicUpdate)
  global.state_event.on('configUpdated', handleConfigUpdate)
}
