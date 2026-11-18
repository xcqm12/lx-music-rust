import searchMusicState, { type Source } from '@/store/search/music/state'
import searchMusicActions, { type SearchResult } from '@/store/search/music/action'
import musicSdk from '@/utils/musicSdk'
import { deviceProfile } from '@/utils/nativeModules/utils'
import { breadcrumb, formatError } from '@/utils/diagnostics'

export const setSource: typeof searchMusicActions['setSource'] = (source) => {
  searchMusicActions.setSource(source)
}
export const setSearchText: typeof searchMusicActions['setSearchText'] = (text) => {
  searchMusicActions.setSearchText(text)
}
export const setListInfo: typeof searchMusicActions['setListInfo'] = (result, id, page) => {
  return searchMusicActions.setListInfo(result, id, page)
}

export const clearListInfo: typeof searchMusicActions['clearListInfo'] = (source) => {
  searchMusicActions.clearListInfo(source)
}

const emptyResult = (source: Source): SearchResult => ({
  allPage: 1,
  limit: 30,
  list: [],
  source: source as SearchResult['source'],
  total: 0,
})

/** 单源搜索；源缺失或抛出异常时返回空结果，避免一个源拖垮整次搜索 */
const searchSource = async(text: string, page: number, limit: number, source: Source): Promise<SearchResult> => {
  try {
    const sdk = musicSdk[source as LX.OnlineSource]
    if (!sdk?.musicSearch?.search) throw new Error('source not found: ' + source)
    return await (sdk.musicSearch.search(text, page, limit) as Promise<SearchResult>)
  } catch (err) {
    console.log(`[search] ${source} failed:`, formatError(err))
    return emptyResult(source)
  }
}

export const search = async(text: string, page: number, sourceId: Source): Promise<LX.Music.MusicInfoOnline[]> => {
  const listInfo = searchMusicState.listInfos[sourceId]!
  if (!text) return []
  const key = `${page}__${text}`
  // 不记录搜索词内容，只记录长度/来源/页数，避免隐私进入日志
  breadcrumb('search.start', `source=${sourceId} page=${page} kwLen=${text.length}`)
  try {
    if (sourceId == 'all') {
      listInfo.key = key
      const sources = searchMusicState.sources.filter(source => source != 'all')
      // 低内存车机（2G 内存）串行请求，避免 5 个响应同时解码/解析造成内存峰值
      let results: SearchResult[]
      if (deviceProfile.isLowRamDevice) {
        results = []
        for (const source of sources) {
          results.push(await searchSource(text, page, searchMusicState.listInfos.all.limit, source))
        }
      } else {
        results = await Promise.all(sources.map(async source => searchSource(text, page, searchMusicState.listInfos.all.limit, source)))
      }
      if (key != listInfo.key) return []
      setSearchText(text)
      setSource(sourceId)
      const list = setListInfo(results, page, text)
      breadcrumb('search.done', `source=all page=${page} count=${list.length}`)
      return list
    } else {
      if (listInfo?.key == key && listInfo?.list.length) return listInfo?.list
      listInfo.key = key
      const sdk = musicSdk[sourceId as LX.OnlineSource]
      if (!sdk?.musicSearch?.search) throw new Error('source not found: ' + sourceId)
      try {
        const data = await (sdk.musicSearch.search(text, page, listInfo.limit) as Promise<SearchResult>)
        if (key != listInfo.key) return []
        const list = setListInfo(data, page, text)
        breadcrumb('search.done', `source=${sourceId} page=${page} count=${list.length}`)
        return list
      } catch (err) {
        if (listInfo.list.length && page == 1) clearListInfo(sourceId)
        throw err
      }
    }
  } catch (err) {
    breadcrumb('search.error', `source=${sourceId} page=${page} ${formatError(err)}`)
    throw err
  }
}
