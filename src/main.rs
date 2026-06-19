// rnr — @antfu-collective/ni の `nr` を Rust で再実装する練習プロジェクト。
// ロードマップは PLAN.md を参照。

use std::process::exit;

fn main() -> anyhow::Result<()> {
    // 先頭の実行ファイル名を捨てて残りの引数を集める。
    let args: Vec<String> = std::env::args().skip(1).collect();

    // 引数なし → 対話選択は Phase 2。今は使い方を出して終了。
    let Some((script, extra)) = args.split_first() else {
        eprintln!("rnr: 実行する script を指定してください（例: rnr build）");
        exit(1);
    };

    let cwd = std::env::current_dir()?;

    // lockfile から package manager を検出。
    let agent = rnr::detect::detect(&cwd);

    // package.json の scripts を読む。
    let pkg = rnr::package::read(&cwd)?;

    // 指定された script が存在しなければエラー終了。
    if !pkg.scripts.contains_key(script) {
        eprintln!("rnr: script '{script}' が package.json に見つかりません");
        exit(1);
    }

    // <bin> run <script> [extra...] を組み立てて実行し、exit code を透過する。
    let argv = rnr::command::build_run_args(agent, script, extra);
    let code = rnr::runner::run(&argv)?;
    exit(code);
}