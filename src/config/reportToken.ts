// 自动生成，勿手改。
/* eslint-disable */
const p = [0,3,2,1,4]
const k = [[60,17,179,125,194,226,63,225,132,91,98,158,24,108,252,103],[20,237,182,167,119,227,218,64,18,1,155,231,106,49,0,18]]
const s = [[],[],[],[],[]]
function u (o: Uint8Array): string {
  let r = ''
  let i = 0
  while (i < o.length) {
    const b = o[i]
    let c: number
    if (b < 0x80) { c = b; i += 1 } else if (b < 0xe0) { c = ((b & 0x1f) << 6) | (o[i + 1] & 0x3f); i += 2 } else if (b < 0xf0) { c = ((b & 0x0f) << 12) | ((o[i + 1] & 0x3f) << 6) | (o[i + 2] & 0x3f); i += 3 } else { c = ((b & 0x07) << 18) | ((o[i + 1] & 0x3f) << 12) | ((o[i + 2] & 0x3f) << 6) | (o[i + 3] & 0x3f); i += 4 }
    r += String.fromCodePoint(c)
  }
  return r
}
function d (): string {
  const t = new Array(5)
  for (let i = 0; i < 5; i++) t[p[i]] = s[i]
  const all: number[] = ([] as number[]).concat(...t)
  const key: number[] = k[0].concat(k[1])
  const out = new Uint8Array(all.length)
  for (let i = 0; i < all.length; i++) out[i] = all[i] ^ key[i % key.length]
  return u(out)
}
export default function (): string {
  return d()
}

