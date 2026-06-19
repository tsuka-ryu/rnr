#!/usr/bin/env node
// rnr の bin shim。
// 今の OS/arch に対応するプラットフォームパッケージから実バイナリを探し、
// 引数そのまま・stdio 継承で実行して exit code を透過する。
const { spawnSync } = require('node:child_process')

// 今の OS/arch から対応するプラットフォームパッケージ名を決める。
// 例: darwin + arm64 → @rnr/darwin-arm64
const pkg = `@rnr/${process.platform}-${process.arch}`

// そのパッケージ内のバイナリの実パスを解決する。
let bin
try {
  bin = require.resolve(`${pkg}/rnr`)
}
catch {
  console.error(`rnr: お使いの環境 (${process.platform}-${process.arch}) 用のバイナリが見つかりません`)
  console.error(`rnr: ${pkg} がインストールされているか確認してください`)
  process.exit(1)
}

// 引数そのまま・stdio 継承で実行する（runner.rs と同じく出力を流す）。
const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' })

// シグナルで終了した場合は status が null になるので 1 にフォールバック。
process.exit(result.status ?? 1)