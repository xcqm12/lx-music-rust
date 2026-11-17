import { Component, Fragment, type ErrorInfo, type ReactNode } from 'react'
import { StyleSheet, Text, TouchableOpacity, View } from 'react-native'
import { log } from '@/utils/log'

interface Props {
  children: ReactNode
  /** 出错页面标识，写入错误日志便于定位 */
  screen?: string
}

interface State {
  hasError: boolean
  error?: Error
  retryKey: number
}

/**
 * 页面级错误边界：主题切换等场景下若渲染抛错，只降级当前页面而不是让整个 APP 闪退。
 * 错误会写入“设置-错误日志”，并可由自动上报通道提交到 GitHub Issues。
 */
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, retryKey: 0 }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    const screen = this.props.screen ?? 'unknown'
    log.error(`[RENDER] ${screen} ${error.stack ?? error.message}\n${info.componentStack ?? ''}`)
  }

  handleRetry = () => {
    this.setState(prev => ({ hasError: false, error: undefined, retryKey: prev.retryKey + 1 }))
  }

  render() {
    if (!this.state.hasError) {
      return <Fragment key={this.state.retryKey}>{this.props.children}</Fragment>
    }
    return (
      <View style={styles.container}>
        <Text style={styles.title}>页面渲染出错</Text>
        <Text style={styles.message} numberOfLines={8}>{this.state.error?.message ?? '未知错误'}</Text>
        <TouchableOpacity style={styles.button} activeOpacity={0.7} onPress={this.handleRetry}>
          <Text style={styles.buttonText}>重试</Text>
        </TouchableOpacity>
        <Text style={styles.tip}>错误已记录到“设置-错误日志”</Text>
      </View>
    )
  }
}

// 不使用主题色：错误可能正来自主题数据异常
const styles = StyleSheet.create({
  container: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 24,
    backgroundColor: '#1f1f1f',
  },
  title: {
    color: '#ffffff',
    fontSize: 17,
    fontWeight: '600',
    marginBottom: 12,
  },
  message: {
    color: '#d0d0d0',
    fontSize: 12,
    marginBottom: 20,
  },
  button: {
    paddingHorizontal: 28,
    paddingVertical: 9,
    borderRadius: 4,
    backgroundColor: '#4daf7c',
  },
  buttonText: {
    color: '#ffffff',
    fontSize: 14,
  },
  tip: {
    color: '#8c8c8c',
    fontSize: 11,
    marginTop: 16,
  },
})
