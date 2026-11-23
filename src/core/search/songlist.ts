import searchSonglistState, { type Source, type ListInfoItem } from '@/store/search/songlist/state'
import searchSonglistActions, { type SearchResult } from '@/store/search/songlist/action'
import musicSdk from '@/utils/musicSdk'
import { deviceProfile } from '@/utils/nativeModules/utils'
import { breadcrumb, formatError } from '@/utils/diagnostics'

export const setSource: typeof searchSonglistActions['setSource'] = (source) => {
  searchSonglistActions.setSource(source)
}
export const setSearchText: typeof searchSonglistActions['setSearchText'] = (text) => {
  searchSonglistActions.setSearchText(text)
}
const setListInfo: typeof searchSonglistActions.setListInfo = (result, page, text) => {
  return searchSonglistActions.setListInfo(result, page, text)
}

export const clearListInfo: typeof searchSonglistActions.clearListInfo = (source) => {
  searchSonglistActions.clearListInfo(source)
}

/** 单源歌单搜索；源缺失或抛出异常时返回空结果，避免一个源拖垮整次搜索 */
const searchSource = async(text: string, page: number, source: Source, limit: number): Promise<SearchResult> => {
  const onlineSource = source as LX.OnlineSource
  try {
    const sdk = musicSdk[onlineSource]
    if (!sdk?.songList?.search) throw new Error('source not found: ' + source)
    return await (sdk.songList.search(text, page, limit) as Promise<SearchResult>)
  } catch (err) {
    console.log(`[songlist search] ${source} failed:`, formatError(err))
    return {
      list: [],
      total: 0,
      limit,
      source: onlineSource,
    }
  }
}

export const search = async(text: string, page: number, sourceId: Source): Promise<ListInfoItem[]> => {
  const listInfo = searchSonglistState.listInfos[sourceId]!
  // if (!text) return []
  const key = `${page}__${sourceId}__${text}`
  // 不记录搜索词内容，只记录长度/来源/页数，避免隐私进入日志
  breadcrumb('songlist.start', `source=${sourceId} page=${page} kwLen=${text.length}`)
  try {
    if (sourceId == 'all') {
      if (listInfo.key == key && listInfo.list.length) return listInfo.list
      listInfo.key = key
      const sources = searchSonglistState.sources.filter(source => source != 'all' && (page <= 1 || page <= (searchSonglistState.maxPages[source] ?? Infinity)))
      // 低内存车机（2G 内存）串行请求，避免多个响应同时解码/解析造成内存峰值（与歌曲搜索一致）
      let results: SearchResult[]
      if (deviceProfile.isLowRamDevice) {
        results = []
        for (const source of sources) {
          results.push(await searchSource(text, page, source, searchSonglistState.listInfos.all.limit))
        }
      } else {
        results = await Promise.all(sources.map(async source => searchSource(text, page, source, searchSonglistState.listInfos.all.limit)))
      }
      if (key != listInfo.key) return []
      setSearchText(text)
      setSource(sourceId)
      const list = setListInfo(results, page, text)
      breadcrumb('songlist.done', `source=all page=${page} count=${list.length}`)
      return list
    } else {
      if (listInfo?.key == key && listInfo?.list.length) return listInfo?.list
      listInfo.key = key
      const data = await searchSource(text, page, sourceId, listInfo.limit)
      if (key != listInfo.key) return []
      const list = setListInfo(data, page, text)
      breadcrumb('songlist.done', `source=${sourceId} page=${page} count=${list.length}`)
      return list
    }
  } catch (err) {
    breadcrumb('songlist.error', `source=${sourceId} page=${page} ${formatError(err)}`)
    if (listInfo.list.length && page == 1) clearListInfo(sourceId)
    throw err
  }
}
