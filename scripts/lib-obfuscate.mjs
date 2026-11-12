/**
 * 凭证混淆共享库（gen-report-token.mjs / gen-report-relay.mjs 共用）
 *
 * 方案：UTF-8 字节 XOR 32 字节随机密钥（密钥拆两半存放）→ 切 5 片乱序
 * （Fisher-Yates，置换表单独存放）→ 生成 TS 模块，变量单字母化，
 * 文件内不含可被正则扫描的特征符号。
 *
 * 注意：混淆只能防自动扫描，挡不住有意逆向；止损靠最小权限与可轮换。
 */
import { randomBytes } from 'node:crypto'

// 与生成文件内的解码器保持一致的实现（自测用）
export function decodeObfuscated(p, k, s) {
  const parts = new Array(5)
  for (let i = 0; i < 5; i++) parts[p[i]] = s[i]
  const all = [].concat(...parts)
  const key = k[0].concat(k[1])
  const out = new Uint8Array(all.length)
  for (let i = 0; i < all.length; i++) out[i] = all[i] ^ key[i % key.length]
  // 手写 UTF-8 解码（Hermes 无 TextDecoder）
  let str = ''
  let i = 0
  while (i < out.length) {
    const b = out[i]
    let cp
    if (b < 0x80) { cp = b; i += 1 } else if (b < 0xe0) { cp = ((b & 0x1f) << 6) | (out[i + 1] & 0x3f); i += 2 } else if (b < 0xf0) { cp = ((b & 0x0f) << 12) | ((out[i + 1] & 0x3f) << 6) | (out[i + 2] & 0x3f); i += 3 } else { cp = ((b & 0x07) << 18) | ((out[i + 1] & 0x3f) << 12) | ((out[i + 2] & 0x3f) << 6) | (out[i + 3] & 0x3f); i += 4 }
    str += String.fromCodePoint(cp)
  }
  return str
}

export function obfuscate(text) {
  const raw = Buffer.from(text, 'utf8')
  const key = randomBytes(32)
  const enc = Buffer.alloc(raw.length)
  for (let i = 0; i < raw.length; i++) enc[i] = raw[i] ^ key[i % 32]

  // 切成 5 片并乱序
  const CHUNKS = 5
  const base = Math.floor(raw.length / CHUNKS)
  const sizes = Array.from({ length: CHUNKS }, () => base)
  for (let i = 0; i < raw.length % CHUNKS; i++) sizes[i]++
  const chunks = []
  let offset = 0
  for (const size of sizes) {
    chunks.push(Array.from(enc.subarray(offset, offset + size)))
    offset += size
  }
  const order = [0, 1, 2, 3, 4]
  for (let i = CHUNKS - 1; i > 0; i--) {
    const j = key[i] % (i + 1)
    ;[order[i], order[j]] = [order[j], order[i]]
  }
  const shuffled = order.map(i => chunks[i])
  return { p: order, k: [Array.from(key.subarray(0, 16)), Array.from(key.subarray(16))], s: shuffled }
}

/** 生成 TS 模块文本；postlude 里可用 d() 取得解码后的字符串 */
export function renderModule(postlude) {
  const data = obfuscate(postlude.__text)
  // roundtrip 自测
  if (decodeObfuscated(data.p, data.k, data.s) !== postlude.__text) {
    throw new Error('roundtrip 自测失败')
  }
  const arr = a => `[${a.join(',')}]`
  const arrs = list => `[${list.map(arr).join(',')}]`
  return `// 自动生成，勿手改。
/* eslint-disable */
const p = ${arr(data.p)}
const k = ${arrs(data.k)}
const s = ${arrs(data.s)}
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
${postlude.body}
`
}

/** 便捷封装：直接渲染"解码并返回字符串"的默认导出 */
export function renderStringModule(text) {
  return renderModule({ __text: text, body: 'export default function (): string {\n  return d()\n}\n' })
}
