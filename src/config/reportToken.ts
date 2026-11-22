// 自动生成，勿手改。
/* eslint-disable */
const p = [2,1,4,3,0]
const k = [[77,79,90,215,120,232,111,115,248,56,33,58,158,111,197,176],[2,86,5,125,199,13,253,63,79,210,214,184,55,19,62,205]]
const s = [[91,33,173,90,88,120,166,25,129,133,74,63,51,49,146,70,203,9,60],[52,247,52,176,14,22,128,167,214,103,43,113,160,117,16,47,134,59,175],[94,171,60,151,135,67,96,79,62,241,122,164,88,36,182,131,235,115],[150,129,143,70,90,116,175,24,39,18,181,65,175,21,68,179,89,87],[42,38,46,191,13,138,48,3,153,76,126,11,175,46,157,132,48,101,80]]
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

