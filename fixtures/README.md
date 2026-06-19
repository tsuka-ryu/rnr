# fixtures — 手元で rnr を試すための置き場

各ディレクトリに package.json と lockfile を1種類だけ置いてある。
`rnr` を実行すると、そのディレクトリの lockfile から package manager が検出される。

| dir | lockfile | 検出される Agent |
| --- | --- | --- |
| `pnpm/` | pnpm-lock.yaml | Pnpm |
| `npm/`  | package-lock.json | Npm |
| `yarn/` | yarn.lock | Yarn |
| `bun/`  | bun.lockb | Bun |

## 使い方

ビルドしてから fixture ディレクトリで実行する:

```sh
cargo build
cd fixtures/pnpm
../../target/debug/rnr hello     # → pnpm run hello → "hi-from-script"
../../target/debug/rnr boom      # → exit code 7 が透過される
../../target/debug/rnr nope      # → script が無いのでエラー終了
```

注意:
- 実際に exec するので、検出された PM（pnpm / yarn / bun など）が
  インストールされていないとそのコマンド実行で失敗する。
- Phase 1 で `--dry-run` を入れたら、PM 未インストールでも
  「組み立てた argv」だけ確認できるようになる（検出ロジックの確認用に便利）。