// rnr — @antfu-collective/ni の `nr` を Rust で再実装する練習プロジェクト。
// ロードマップは PLAN.md を参照。

use std::process::exit;

fn main() -> anyhow::Result<()> {
    // 先頭の実行ファイル名を捨てて残りの引数を集める。
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    // --dry-run があれば取り除いてフラグを立てる（位置はどこでもよい）。
    // 本家の debug モード相当: 実行せず組み立てた argv を表示する。
    let dry_run = args.iter().any(|a| a == "--dry-run");
    args.retain(|a| a != "--dry-run");

    let cwd = std::env::current_dir()?;

    // lockfile から package manager を検出し、package.json の scripts を読む。
    let agent = rnr::detect::detect(&cwd);
    let pkg = rnr::package::read(&cwd)?;

    // 直前に実行した script を読み込む（rnr - とピン留めに使う）。
    let mut storage = rnr::storage::load();

    // 実行する script と追加引数を決める。
    let (script, extra): (String, Vec<String>) = match args.split_first() {
        // 引数なし → fuzzy 選択 UI。直前 script は先頭にピン留めされる。
        None => match rnr::prompt::select_script(&pkg.scripts, storage.last_run_command.as_deref())?
        {
            Some(s) => (s, Vec::new()),
            // ESC / Ctrl-C でキャンセル → 静かに終了。
            None => exit(1),
        },
        // rnr - → 直前に実行した script を再実行。
        Some((first, rest)) if first == "-" => match storage.last_run_command.clone() {
            Some(s) => (s, rest.to_vec()),
            None => {
                eprintln!("rnr: 直前に実行した script がありません");
                exit(1);
            }
        },
        // 通常: 先頭が script、残りが追加引数。
        Some((first, rest)) => (first.clone(), rest.to_vec()),
    };

    // 指定された script が存在しなければエラー終了。
    if !pkg.scripts.contains_key(&script) {
        eprintln!("rnr: script '{script}' が package.json に見つかりません");
        exit(1);
    }

    // <bin> run <script> [extra...] を組み立てる。
    let argv = rnr::command::build_run_args(agent, &script, &extra);

    // --dry-run なら実行せず argv を1行表示して終了（状態は変更しない）。
    if dry_run {
        println!("{}", argv.join(" "));
        exit(0);
    }

    // 直前と違う script なら lastRunCommand を更新して保存。
    // 保存失敗は致命的でないので握りつぶす（キャッシュ方針）。
    if storage.last_run_command.as_deref() != Some(script.as_str()) {
        storage.last_run_command = Some(script.clone());
        let _ = rnr::storage::save(&storage);
    }

    // 実行して exit code を透過する。
    let code = rnr::runner::run(&argv)?;
    exit(code);
}
