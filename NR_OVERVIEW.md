# `nr` の処理の全体像

`nr` は package.json の `scripts` を、検出したパッケージマネージャ（npm / yarn / pnpm / bun など）に合わせて実行するコマンド。
`nr dev` なら `npm run dev` / `pnpm run dev` / `yarn dev` のように、適切な run コマンドへ変換して実行する。

主要なファイル:

- [src/commands/nr.ts](src/commands/nr.ts) — `nr` コマンドのエントリポイント、対話選択や特殊フラグの処理
- [src/runner.ts](src/runner.ts) — 共通の実行基盤（`runCli` / `run` / `getCliCommand`）
- [src/parse.ts](src/parse.ts) — agent + args から実行コマンドへ変換（`parseNr` など）
- [src/package.ts](src/package.ts) — package.json から scripts 一覧を読む
- [src/config.ts](src/config.ts) — 設定（`runAgent`, `noLastCommand`, `defaultAgent` など）
- [src/storage.ts](src/storage.ts) — `lastRunCommand` の永続化

---

## 全体フロー

```
nr <args>
  │
  ▼
runCli(fn, options)                         [runner.ts]
  │  環境変数オプションを merge し run() を呼ぶ
  ▼
run(fn, args, options)                      [runner.ts]
  │  1. 特殊フラグの前処理:
  │     - `?`            → debug（dry-run）モード
  │     - `--programmatic` → programmatic モード
  │     - `-C <dir>`     → cwd 変更
  │     - `-v/--version` → バージョン表示して終了
  │     - `--agent`      → 検出した agent 名のみ出力して終了
  │     - `-h/--help`    → ヘルプ表示して終了
  │  2. onBeforeCommand フック:
  │     - `--completion-zsh/bash/fish` → 補完スクリプトを出力して終了
  │  3. getCliCommand() で agent を決定し fn(=nr本体) を実行
  ▼
getCliCommand(fn, args)                     [runner.ts]
  │  - `-g` があれば global agent
  │  - それ以外は detect() でロックファイルから検出
  │    → なければ defaultAgent（設定 or 'prompt'）
  │  - 'prompt' の場合は対話で agent を選択
  ▼
nr 本体の Runner                            [commands/nr.ts]
  │  （詳細は下記）
  │  最終的に parseNr() で ResolvedCommand を返す
  ▼
run() に戻り、コマンドを実行                [runner.ts]
  │  - sfw / volta によるラップ（設定・存在時）
  │  - debug モードならコマンド文字列を出力して終了
  │  - tinyexec の x() で stdio:'inherit' で実行
  ▼
子プロセスとして script を実行
```

---

## `nr` 本体（[commands/nr.ts](src/commands/nr.ts)）の分岐

`runCli` のコールバック内で、agent 決定後に args を見て処理する。

1. **`--completion`**
   シェル補完の候補生成。bash では `COMP_LINE` / `COMP_CWORD` を見て、`nr <ここ>` の位置だけ候補を返す。他シェルは候補をそのまま出力。処理後 return。

2. **`-p`（monorepo / workspace のスクリプト）**
   [readWorkspaceScripts](src/package.ts#L11) で workspace のパッケージを選択 → そのパッケージの scripts を取得。複数あれば対話選択プロンプトを表示。選択したパッケージに `ctx.cwd` を切り替える。

3. **`-`（直前のコマンドを再実行）**
   `storage.lastRunCommand` を `args[0]` に展開。記録がなければエラー終了。

4. **引数なし（`nr` 単体）**
   `programmatic` でなければ [readPackageScripts](src/package.ts#L31) で scripts 一覧を取得し、対話選択プロンプト（`promptSelectScript`）を表示。選んだ script を `args` に push する。

5. **`lastRunCommand` の更新**
   `args[0]` が直前と違えば `storage.lastRunCommand` に保存して `dump()` で永続化。

6. **`parseNr(agent, args, ctx)` を返す** → コマンドへ変換。

### 対話選択プロンプト（`promptSelectScript`）

- package.json の scripts を `@posva/prompts` の autocomplete で表示。
- 設定 `noLastCommand` が false で、直前に実行した script があれば一覧の先頭に並べる。
- [fzf](src/commands/nr.ts#L34) によるあいまい検索（key + description 対象、大文字小文字無視）。
- 各候補の説明文（description）は端末幅に合わせて `limitText` で省略。
- ESC で抜けた場合は `isExited` フラグで検知して `process.exit(1)`（prompts のバグ回避）。

---

## `parseNr`（[parse.ts](src/parse.ts#L55)）— args → 実行コマンド

agent と args から、実際に走らせる `{ command, args, cwd }`（ResolvedCommand）を組み立てる。

- **引数なし** → `start` を補う（`nr` → `npm start` 相当）。
- **`runAgent === 'node'`（設定）** → Node の `--run` で直接実行。Node 22 未満ならエラー。
- **`--if-present`** → 一旦除去し、`node` 実行でなければ後で run の直後に差し込む。
- **`-p`** → 残っていれば除去（実行コマンドには渡さない）。
- **workspace フラグの正規化** → `-w value` / `--workspace value` を `-w=value` /
  `--workspace=value` に結合（npm がフラグを boolean と誤認するのを防ぐ）。
- 最終的に [getCommand(agent, 'run', args)](src/parse.ts#L13) で agent ごとの run コマンドテンプレートに当てはめる（`runWithNode` の場合は `node` を直接使う）。
- `ctx.cwd` があれば（`-p` で選んだパッケージなど）コマンドの `cwd` に設定。

---

## description（説明文）の解決順

[readPackageScripts](src/package.ts#L31) では、各 script の説明を次の優先順で決める:

1. `package.json` の `scripts-info[key]`（[npm-scripts-info](https://www.npmjs.com/package/npm-scripts-info) 規約）
2. `scripts["?<key>"]`（`?` プレフィックスの説明用スクリプト）
3. なければコマンド本文（`cmd`）そのもの

`?` で始まる key は実行候補一覧からは除外される（説明専用のため）。

---

## 実行段階の補足（[runner.ts](src/runner.ts) 終盤）

- **sfw**: 設定 `useSfw` が有効で `sfw` が存在すれば、コマンドを `sfw <command> ...` でラップ。有効だが未インストールならエラー。
- **volta**: `volta` が存在すれば `volta run <command> ...` でラップ。
- **debug（`?`）**: 実行せず、解決後のコマンド文字列を出力して終了（dry-run）。
- **実行**: `tinyexec` の `x()` を `stdio: 'inherit'` で実行。`SIGINT` を受けても子プロセスのクリーンアップ完了を待ってから終了コードを引き継ぐ。
