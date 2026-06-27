/**
 * SafeText - 安全文本组件
 * 强制将传入内容转换为字符串显示，规避 RN Text 组件的校验报错
 * 主要用于显示来自用户音源（WebView 沙箱）的文本内容
 */

import { memo, type ComponentProps } from 'react'
import { Text, type TextProps as _TextProps, StyleSheet, type ColorValue } from 'react-native'
import { useTextShadow, useTheme } from '@/store/theme/hook'
import { setSpText } from '@/utils/pixelRatio'

export interface SafeTextProps extends _TextProps {
  /** 字体大小 */
  size?: number
  /** 字体颜色 */
  color?: ColorValue
}

/**
 * 将任意值转换为安全字符串
 * undefined/null -> 空字符串
 * 对象/数组 -> JSON 字符串
 * 其他 -> String()
 */
const toSafeString = (value: unknown): string => {
  if (value === undefined || value === null) return ''
  if (typeof value === 'object') {
    try {
      return JSON.stringify(value)
    } catch {
      return String(value)
    }
  }
  return String(value)
}

/**
 * SafeText 组件
 * - 强制将 children 转换为字符串
 * - 自动处理 undefined/null/对象/数组等情况
 * - 避免 RN Text 组件的报错
 */
const SafeText = memo(({
  style,
  size = 15,
  color,
  children,
  ...props
}: SafeTextProps) => {
  const theme = useTheme()
  const textShadow = useTextShadow()

  // 强制转换 children 为安全字符串
  const safeChildren = toSafeString(children)

  style = StyleSheet.compose(textShadow ? {
    textShadowColor: theme['c-primary-dark-300-alpha-800'],
    textShadowOffset: { width: 0.2, height: 0.2 },
    textShadowRadius: 2,
    fontSize: setSpText(size),
    color: color ?? theme['c-font'],
  } : {
    fontSize: setSpText(size),
    color: color ?? theme['c-font'],
  }, style)

  return (
    <Text
      style={style}
      {...props}
    >{safeChildren}</Text>
  )
})

SafeText.displayName = 'SafeText'

export default SafeText
