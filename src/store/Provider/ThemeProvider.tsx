import { memo, useEffect, useState } from 'react'

import themeState, { ThemeContext } from '../theme/state'


export default memo(({ children }: {
  children: React.ReactNode
}) => {
  const [theme, setTheme] = useState(themeState.theme)

  useEffect(() => {
    const handleUpdateTheme = (theme: LX.ActiveTheme) => {
      // 只接受字段完整的主题对象，避免把 undefined 颜色传给整棵页面树
      if (!theme?.id || typeof theme['c-font'] != 'string' || typeof theme['c-content-background'] != 'string') return
      requestAnimationFrame(() => {
        setTheme(theme)
      })
    }
    global.state_event.on('themeUpdated', handleUpdateTheme)
    return () => {
      global.state_event.off('themeUpdated', handleUpdateTheme)
    }
  }, [])

  return (
    <ThemeContext.Provider value={theme}>
      {children}
    </ThemeContext.Provider>
  )
})
