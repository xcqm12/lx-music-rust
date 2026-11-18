import { getMemoryInfo, recordBreadcrumb } from '@/utils/nativeModules/utils'

/**
 * 诊断面包屑（轻量）：把关键操作写入原生侧日志，进程被 OOM 杀死或原生崩溃后，
 * 下次启动会把最后操作与内存快照并入“设置-错误日志”，再由自动上报提交。
 * 不记录搜索词等隐私内容，只记录长度/来源/页数等定位信息。
 */
export const breadcrumb = (tag: string, detail = '') => {
  recordBreadcrumb(tag, detail)
}

/** 记录带内存快照的面包屑（异步取原生内存信息） */
export const breadcrumbWithMemory = (tag: string) => {
  void getMemoryInfo().then(info => {
    if (!info) return
    breadcrumb(tag, `avail=${info.availMemMB ?? '?'}MB low=${String(info.lowMemory ?? '?')} heap=${info.heapUsedMB ?? '?'}/${info.heapMaxMB ?? '?'}MB`)
  })
}

export const formatError = (err: unknown) => {
  if (err instanceof Error) return `${err.name}: ${err.message}`
  return String(err)
}
