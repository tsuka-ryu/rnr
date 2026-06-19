# rnr — Rust 版 `nr`

[`@antfu-collective/ni`](https://github.com/antfu-collective/ni) の `nr`（package.json の script 実行）を
Rust で再実装する**個人練習プロジェクト**。

目的は実用ツールで本家を置き換えることではなく、

1. Rust の言語そのもの（所有権 / `Result` / `match` / 文字列の取り回し）に慣れる
2. **Rust製CLIを npm エコシステムへ配布する**構造（oxlint / esbuild 方式）を理解する

の2点を体で覚えること。

## 進め方（このプロジェクトの作業前提）

- **実装は本人が手で書く。** Claude（Opus）は実装コードを書かず、**ステップ単位の仕様だけ**を出す。
- **細切りで進める。** 1 ステップぶんの仕様を渡す → 本人が書く → 見せてレビュー → 次へ、のリズム。
  Phase をまとめて渡さない。
- **Rust の文法ヒントは不要。** `match` / `Option` / 所有権などの
  解説は省く。指示は「関数のシグネチャ・振る舞い・本家との差分・エッジケース」に絞る。
- 各ステップは `cargo build` が通ることを確認してから次へ。
- 見積もり: コア（Phase 0〜3）5〜7h + 軽量配布（Phase 4）0.5〜1h ≒ 計 6〜8h。

## `nr` の本質（移植する振る舞い）

1. **lockfile から package manager を検出**（npm / pnpm / yarn / bun）
2. **package.json の `scripts` を読む**
3. 引数あり → `<pm> run <script> [extra args]` を組んで **exec（stdio 継承・exit code 透過）**
4. 引数なし → **fuzzy な対話選択 UI** で script を選ぶ（`nr` で一番使う機能）
5. `nr -` → **直前に実行した script を再実行**（storage に保存した lastRunCommand）
6. 実行直前に **volta ラッパー**判定（ここに mise 改善を入れる / 下記）

後回しでよいもの: shell completion 生成、`-p`（monorepo workspace）、`runAgent: node`。

## 構成（2レイヤー）

```
rnr/
├── Cargo.toml
├── PLAN.md
├── src/
│   ├── main.rs       # 引数パース → dispatch
│   ├── detect.rs     # lockfile 検出 → Agent enum
│   ├── package.rs    # package.json の scripts を読む（serde）
│   ├── command.rs    # Agent + script → argv 組み立て（+ volta/mise）
│   ├── runner.rs     # std::process::Command で exec、exit code 透過
│   ├── storage.rs    # lastRunCommand の保存/読み込み
│   └── prompt.rs     # 対話 fuzzy 選択（Phase 2）
└── npm/              # 配布レイヤー（Phase 4・軽量版）
    ├── rnr/          # 本体パッケージ（optionalDependencies + bin shim）
    └── platforms/    # @rnr/<os>-<arch>（まずは自分の 1 ターゲットだけ）
```

### 依存（最小から）

| 用途 | crate | 入れる時期 |
| --- | --- | --- |
| エラー | `anyhow` | Phase 0（導入済み） |
| package.json パース | `serde` + `serde_json` | Phase 0（導入済み） |
| 子プロセス exec | `std::process::Command` | 標準（依存なし） |
| 引数パース | 手書き `std::env::args`（nr の引数処理は単純なので clap は基本不要） | Phase 0 |
| 対話 fuzzy 選択 | `inquire`（fuzzy filter 内蔵の `Select`） | Phase 2 |

> 最初は依存を `anyhow` + `serde` だけに絞り、言語そのものに集中する。

## フェーズ

### Phase 0 — 動く最小 `rnr <script>`
- `Agent` enum（`Npm` / `Pnpm` / `Yarn` / `Bun`）
- lockfile 検出のみ: `pnpm-lock.yaml`→Pnpm, `yarn.lock`→Yarn, `bun.lockb`/`bun.lock`→Bun,
  `package-lock.json`→Npm、無ければ Npm にフォールバック
- `package.json` の `scripts` を serde で読む
- `rnr build` → `pnpm run build` を `Command::new(...).status()` で exec、exit code をそのまま返す
- **ゴール**: `rnr <既存 script>` が本物のように動く

### Phase 1 — `--dry-run`（テスト可能化）
- 実行せず「組み立てた argv」を print するフラグ（本家の `?` 相当）
- `tests/` に「lockfile + args → 期待 argv」のテストを書く
- **ゴール**: `cargo test` で移植の正しさを担保

### Phase 2 — 対話 fuzzy 選択 + `rnr -`（コア機能）
- `inquire` の `Select` で `key - description` 一覧を fuzzy 絞り込み → 実行
- **lastRunCommand を先頭にピン留め**（本家挙動）。ESC でキャンセル
- `storage.rs`: 選んだ script を一時ファイル（`~/.cache` 等）に保存、`rnr -` で読む
- **ゴール**: 日常使いできる UX（スクショの候補リストを再現）

### Phase 3 — volta / mise（本家にない自分の理想挙動）
- 実行直前に: `volta` が PATH にあり、**mise が active でなければ** `volta run <cmd>` でラップ
- mise active 判定は環境変数（`MISE_SHELL` / `__MISE_DIFF` など）で
- **ゴール**: volta が入っていても mise を使っていれば volta に横取りされない

### Phase 4 — npm 配布の「仕組み」を軽量に再現（oxlint / esbuild 方式）

CI のクロスビルド地獄（musl / win-msvc / マトリクス publish）は**やらない**。
試行錯誤コストが高く、Rust 力では短縮できないため。配布の**構造を手で1周組む**ことだけを目的にする。

- **ローカルの自分のターゲットだけビルド**: `cargo build --release`（例: mac arm64 の 1 個）
- その 1 バイナリを `@rnr/<os>-<arch>` パッケージの形に置く（package.json に `os` / `cpu` を指定）
- 本体 `rnr` パッケージが `optionalDependencies` で参照、`bin` の shim が該当バイナリを
  `require.resolve` して exec する**構造を手で組む**
- **ゴール**: 「optionalDependencies + bin shim」の配布パターンを 1 ターゲットで再現して理解する。
  publish は任意（やるなら手動 1 回）。

> フルマトリクス配布（全 OS/arch を CI でビルドして publish）は後回し。1 個組めれば残りは
> "同じことを並べるだけ" なので、学習目的としては単一ターゲットで十分。

## 参考

- 移植元の仕様: [antfu-collective/ni `src/commands/nr.ts`](https://github.com/antfu-collective/ni/blob/main/src/commands/nr.ts)
- PM 検出ロジックの正解表: [package-manager-detector](https://github.com/antfu-collective/package-manager-detector)
- npm 配布の教科書: [oxc-project/oxc](https://github.com/oxc-project/oxc) の oxlint パッケージ + `.github/workflows`
