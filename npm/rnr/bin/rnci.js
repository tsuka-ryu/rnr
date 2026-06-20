#!/usr/bin/env node
// rnci の bin shim（busybox 方式）。
// 実体は rnr と同じバイナリ。argv0 を 'rnci' に差し替えて spawn することで、
// バイナリ側の argv[0] dispatch が frozen install 経路を選ぶ。
const { spawnSync } = require('node:child_process')

const pkg = `@rnr/${process.platform}-${process.arch}`

let bin
try {
  bin = require.resolve(`${pkg}/rnr`)
}
catch {
  console.error(`rnci: お使いの環境 (${process.platform}-${process.arch}) 用のバイナリが見つかりません`)
  console.error(`rnci: ${pkg} がインストールされているか確認してください`)
  process.exit(1)
}

// argv0='rnci' で呼ぶ。これでバイナリ側が frozen install に分岐する。
const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit', argv0: 'rnci' })

process.exit(result.status ?? 1)