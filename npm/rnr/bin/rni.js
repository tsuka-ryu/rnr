#!/usr/bin/env node
// rni の bin shim（busybox 方式）。
// 実体は rnr と同じバイナリ。argv0 を 'rni' に差し替えて spawn することで、
// バイナリ側の argv[0] dispatch が ni 経路を選ぶ（main.rs の program_name 参照）。
const { spawnSync } = require('node:child_process')

// 今の OS/arch から対応するプラットフォームパッケージ名を決める。
const pkg = `@rnr/${process.platform}-${process.arch}`

// そのパッケージ内のバイナリの実パスを解決する。
let bin
try {
  bin = require.resolve(`${pkg}/rnr`)
}
catch {
  console.error(`rni: お使いの環境 (${process.platform}-${process.arch}) 用のバイナリが見つかりません`)
  console.error(`rni: ${pkg} がインストールされているか確認してください`)
  process.exit(1)
}

// argv0='rni' で呼ぶ。これでバイナリ側が ni 経路に分岐する。
const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit', argv0: 'rni' })

// シグナルで終了した場合は status が null になるので 1 にフォールバック。
process.exit(result.status ?? 1)