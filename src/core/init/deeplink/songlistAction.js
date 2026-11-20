import { playSonglist } from './playSonglist'
import { dataVerify, sourceVerify } from './utils'

const handlePlaySonglist = async({ paths, data }) => {
  let songlistInfo = {
    source: null,
    id: null,
    url: null,
    index: null,
  }
  if (data) {
    songlistInfo = data
  } else {
    songlistInfo.source = paths[0]
    songlistInfo.url = paths[1]
    songlistInfo.index = paths[2]
    if (songlistInfo.index != null) {
      songlistInfo.index = parseInt(songlistInfo.index)
      if (Number.isNaN(songlistInfo.index)) delete songlistInfo.index
    }
  }

  sourceVerify(songlistInfo.source)

  songlistInfo = dataVerify([
    { key: 'source', types: ['string'] },
    { key: 'id', types: ['string', 'number'], max: 64 },
    { key: 'url', types: ['string'], max: 500 },
    { key: 'index', types: ['number'], max: 1000000 },
  ], songlistInfo)

  if (!songlistInfo.id && !songlistInfo.url) throw new Error('id or url missing')

  await playSonglist(songlistInfo.source, songlistInfo.id ?? songlistInfo.url, songlistInfo.index)
}

export const handleSonglistAction = async(action, info) => {
  switch (action) {
    case 'play':
      await handlePlaySonglist(info)
      break
  }
}
