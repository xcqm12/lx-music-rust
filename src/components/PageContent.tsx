import { View } from 'react-native'
import { useTheme } from '@/store/theme/hook'
import ImageBackground from '@/components/common/ImageBackground'
import { useWindowSize } from '@/utils/hooks'
import { useMemo } from 'react'
import { scaleSizeAbsHR } from '@/utils/pixelRatio'
import { defaultHeaders } from './common/Image'
import SizeView from './SizeView'
import { useBgPic } from '@/store/common/hook'
import SafeText from '@/components/common/SafeText'
import { deviceProfile } from '@/utils/nativeModules/utils'

interface Props {
  children: React.ReactNode
}

const BLUR_RADIUS = Math.max(scaleSizeAbsHR(18), 10)
// 模糊背景需要额外分配整屏位图。车机/Android 6 老设备多为低内存机器，
// 切换主题或动态背景时容易 OOM 闪退，这类设备直接降级为半透明纯色遮罩。
const ENABLE_BLUR = !deviceProfile.isLowRamDevice

export default ({ children }: Props) => {
  const theme = useTheme()
  const windowSize = useWindowSize()
  const pic = useBgPic()

  const memoChildren = useMemo(() => children, [children])
  const renderChildren = ((): React.ReactNode => {
    if (typeof memoChildren === 'string' || typeof memoChildren === 'number') {
      return <SafeText>{memoChildren}</SafeText>
    }
    return memoChildren
  })()

  const themeComponent = useMemo(() => (
    <View style={{ flex: 1, overflow: 'hidden' }}>
      <ImageBackground
        style={{ position: 'absolute', left: 0, top: 0, height: windowSize.height, width: windowSize.width, backgroundColor: theme['c-content-background'] }}
        source={theme['bg-image']}
        resizeMode="cover"
        // 让原生层按视图尺寸降采样，避免把超大主题背景图整张解码进内存
        resizeMethod="resize"
      >
      </ImageBackground>
      <View style={{ flex: 1, flexDirection: 'column', backgroundColor: theme['c-main-background'] }}>
        {renderChildren}
      </View>
    </View>
  ), [renderChildren, theme, windowSize.height, windowSize.width])

  const picComponent = useMemo(() => {
    return (
      <View style={{ flex: 1, overflow: 'hidden' }}>
        <ImageBackground
          style={{ position: 'absolute', left: 0, top: 0, height: windowSize.height, width: windowSize.width, backgroundColor: theme['c-content-background'] }}
          source={{ uri: pic!, headers: defaultHeaders }}
          resizeMode="cover"
          resizeMethod="resize"
          blurRadius={ENABLE_BLUR ? BLUR_RADIUS : undefined}
        >
          <View style={{ flex: 1, flexDirection: 'column', backgroundColor: theme['c-content-background'], opacity: 0.76 }}></View>
        </ImageBackground>
        <View style={{ flex: 1, flexDirection: 'column' }}>
          {renderChildren}
        </View>
      </View>
    )
  }, [renderChildren, pic, theme, windowSize.height, windowSize.width])

  return (
    <>
      <SizeView />
      {pic ? picComponent : themeComponent}
    </>
  )
}
