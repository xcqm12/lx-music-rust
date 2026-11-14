#!/usr/bin/env node
/**
 * 错误日志 SSH 中转凭证混淆文件生成器
 *
 * 用法：
 *   node scripts/gen-report-relay.mjs --host <ip> --port <port> --user <user> \
 *        --key-file <私钥路径> [--host-key-sha256 <指纹>]
 *   LX_REPORT_RELAY='{"host":...,"port":...,"user":...,"key":...,"hostKeySha256":...}' \
 *     node scripts/gen-report-relay.mjs --env          # CI 用
 *   node scripts/gen-report-relay.mjs --allow-empty    # 生成空占位文件（禁用中转）
 *
 * 输出 src/config/reportRelay.ts（混淆方案见 lib-obfuscate.mjs）。
 * hostKeySha256 为服务器主机公钥的 SHA-256 指纹（base64 无填充，OpenSSH 风格），
 * 由原生层在握手后比对，防中间人。中转凭证是 authorized_keys ForceCommand
 * 受限密钥，泄漏后只能在服务器上重新生成密钥对并重新运行本脚本；
 * GitHub PAT 始终只存服务器端。
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { renderModule } from './lib-obfuscate.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))
const OUT_FILE = join(__dirname, '..', 'src', 'config', 'reportRelay.ts')

const args = process.argv.slice(2)
const allowEmpty = args.includes('--allow-empty')
const outFileIdx = args.indexOf('--out')
const outFile = outFileIdx > -1 ? args[outFileIdx + 1] : undefined
const readArg = name => {
  const i = args.indexOf(`--${name}`)
  return i >= 0 ? args[i + 1] : undefined
}
const OUT_PATH = outFile ? join(__dirname, '..', outFile) : OUT_FILE

let config = { host: '', port: 0, user: '', key: '', hostKeySha256: '' }
if (args.includes('--env')) {
  const raw = process.env.LX_REPORT_RELAY ?? ''
  if (raw) config = JSON.parse(raw)
} else if (!allowEmpty) {
  const host = readArg('host')
  const port = Number(readArg('port'))
  const user = readArg('user')
  const keyFile = readArg('key-file')
  if (!host || !port || !user || !keyFile) {
    console.error('错误：参数不完整。用法：--host <ip> --port <port> --user <user> --key-file <路径> [--host-key-sha256 <指纹>] 或 --env / --allow-empty')
    process.exit(1)
  }
  config = {
    host,
    port,
    user,
    key: readFileSync(keyFile, 'utf8'),
    hostKeySha256: readArg('host-key-sha256') ?? '',
  }
}

const text = config.host ? JSON.stringify(config) : ''

// 校验：能还原、能解析、私钥形状正确
if (text) {
  const parsed = JSON.parse(text)
  if (!parsed.host || !parsed.port || !parsed.user || !parsed.key.includes('PRIVATE KEY')) {
    console.error('错误：中转凭证字段不完整（缺少 host/port/user/key 或私钥格式不对）')
    process.exit(1)
  }
}

const body = `export interface RelayConfig {
  host: string
  port: number
  user: string
  key: string
  hostKeySha256: string
}

export default function (): RelayConfig | null {
  try {
    const raw = d()
    if (!raw) return null
    const config = JSON.parse(raw) as RelayConfig
    if (!config.host || !config.port || !config.user || !config.key || !config.hostKeySha256) return null
    return config
  } catch {
    return null
  }
}
`

writeFileSync(OUT_PATH, renderModule({ __text: text, body }))
console.log(`已生成 ${OUT_PATH}（roundtrip 自测通过${text ? '' : '，中转未配置'}）`)
