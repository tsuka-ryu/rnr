# rnr

[`@antfu-collective/ni`](https://github.com/antfu-collective/ni) の `nr`（package.json の
script ランナー）を Rust で再実装する**個人練習プロジェクト**。

> ⚠️ **WIP** — まだ動きません。スタブと方針のみの段階です。

## なぜ作るか

実用ツールで本家を置き換えるためではなく、次の2つを体で覚えるため:

1. Rust の言語そのもの（所有権 / `Result` / `match` / 文字列の取り回し）に慣れる
2. **Rust 製 CLI を npm エコシステムへ配布する**実務パターン（oxlint / esbuild 方式）を一周組む

題材に `nr` を選んだ理由や設計判断は [PLAN.md](./PLAN.md) を参照。

## やること（`nr` の本質）

1. lockfile から package manager を検出（npm / pnpm / yarn / bun）
2. `package.json` の `scripts` を読む
3. `rnr <script>` → `<pm> run <script>` を exec（stdio 継承・exit code 透過）
4. `rnr`（引数なし）→ fuzzy な対話選択で script を選ぶ
5. `rnr -` → 直前に実行した script を再実行
6. volta / mise の解決（mise が active なら volta に横取りさせない）

ロードマップは Phase 0〜4 として [PLAN.md](./PLAN.md) に記載。

## 開発

```sh
cargo run -- <script>   # 実行
cargo test              # テスト（Phase 1 以降）
```

## ライセンス

MIT
