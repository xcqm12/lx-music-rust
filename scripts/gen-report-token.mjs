#!/usr/bin/env node
/**
 * 错误上报直连凭证混淆文件生成器
 *
 * 用法：
 *   node scripts/gen-report-token.mjs <token>          # 从参数读取
 *   LX_REPORT_TOKEN=<token> node scripts/gen-report-token.mjs   # 从环境变量读取（CI 用）
 *   node scripts/gen-report-token.mjs --in <文件路径>   # 从文件读取（自动去 BOM/换行）
 *   node scripts/gen-report-token.mjs --out <相对路径>  # 输出到指定路径（默认 src/config/reportToken.ts）
 *   node scripts/gen-report-token.mjs --allow-empty    # 生成空凭证占位文件
 *
 * 混淆方案见 lib-obfuscate.mjs。注意：中转模式（reportRelay.ts）优先且不需要
 * 此凭证；直连 GitHub API 才会用到，直连凭证只授予 Issues:write 最小权限，
 * 泄漏后换新凭证重新生成本文件即可。
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { renderStringModule } from './lib-obfuscate.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))
const OUT_FILE = join(__dirname, '..', 'src', 'config', 'reportToken.ts')

const args = process.argv.slice(2)
const allowEmpty = args.includes('--allow-empty')
const outFileIdx = args.indexOf('--out')
const outFile = outFileIdx > -1 ? args[outFileIdx + 1] : undefined
const inFileIdx = args.indexOf('--in')
const inFile = inFileIdx > -1 ? args[inFileIdx + 1] : undefined
const positional = args.find(a => !a.startsWith('--') && a !== outFile && a !== inFile)
let token = positional ?? process.env.LX_REPORT_TOKEN ?? ''
if (inFile) token = readFileSync(inFile, 'utf8').replace(/\r\n/g, '\n').replace(/\n+$/, '')

const OUT_PATH = outFile ? join(__dirname, '..', outFile) : OUT_FILE

if (!token && !allowEmpty) {
  console.error('错误：未提供凭证。用法：node scripts/gen-report-token.mjs <token> [--allow-empty]')
  process.exit(1)
}

writeFileSync(OUT_PATH, renderStringModule(token))
console.log(`已生成 ${OUT_PATH}（roundtrip 自测通过${token ? '' : '，凭证为空占位'}）`)
