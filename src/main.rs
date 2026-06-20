// rnr — @antfu-collective/ni を Rust で再実装する練習プロジェクト。
// ロードマップは PLAN.md（nr 系）と PLAN-NI.md（ni 系）を参照。
//
// busybox 方式: 呼ばれた名前（argv[0]）でサブコマンドを振り分ける。
//   rnr  … script 実行（nr 相当）
//   rni  … 依存インストール（ni 相当）
//   rnci … frozen install（nci 相当）
// バイナリは 1 個。npm 配布時は shim が argv0 を差し替えて同じ実体を spawn する。

use std::path::Path;
use std::process::exit;

use rnr::command::{cmd_exists, mise_active, wrap_volta};
use rnr::detect::Agent;
use rnr::install::{build_install_args, InstallSpec};

fn main() -> anyhow::Result<()> {
    // 呼ばれた実行ファイル名（argv[0] のファイル名部分）で経路を決める。
    let prog = program_name();

    // 残りの引数を集め、--dry-run はどの経路でも先に剥がしてフラグ化する。
    // 本家の debug モード相当: 実行せず組み立てた argv を表示する。
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let dry_run = args.iter().any(|a| a == "--dry-run");
    args.retain(|a| a != "--dry-run");

    let cwd = std::env::current_dir()?;
    // lockfile から package manager を検出する（nr / ni 共通）。
    let agent = rnr::detect::detect(&cwd);

    match prog.as_str() {
        // rni: ni 経路。引数を意図に解釈してインストール argv を組む。
        "rni" => run_install(agent, rnr::cli::parse_ni(&args), dry_run),
        // rnci: 名前そのものが frozen install を意味する（引数は見ない）。
        "rnci" => run_install(agent, InstallSpec::Frozen, dry_run),
        // それ以外（rnr など）は従来どおり script 実行。
        _ => run_script(agent, &cwd, args, dry_run),
    }
}

/// argv[0] からファイル名部分だけを取り出す（`/usr/local/bin/rni` → `rni`）。
fn program_name() -> String {
    std::env::args()
        .next()
        .as_deref()
        .map(Path::new)
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// ni 経路: 意図 → argv → volta ラップ → 実行（または --dry-run 表示）。
fn run_install(agent: Agent, spec: InstallSpec, dry_run: bool) -> anyhow::Result<()> {
    // 解決テーブルで argv を組み立てる純粋関数。
    let argv = build_install_args(agent, &spec);

    // nr と同じく volta があり mise 非 active なら volta run でラップする。
    let argv = wrap_volta(argv, cmd_exists("volta"), mise_active());

    if dry_run {
        println!("{}", argv.join(" "));
        exit(0);
    }

    let code = rnr::runner::run(&argv)?;
    exit(code);
}

/// nr 経路: 従来の script 実行。fuzzy 選択・`rnr -`・直前 script 記憶を含む。
fn run_script(agent: Agent, cwd: &Path, args: Vec<String>, dry_run: bool) -> anyhow::Result<()> {
    // package.json の scripts を読む。
    let pkg = rnr::package::read(cwd)?;

    // 直前に実行した script を読み込む（rnr - とピン留めに使う）。
    let mut storage = rnr::storage::load();

    // 実行する script と追加引数を決める。
    let (script, extra): (String, Vec<String>) = match args.split_first() {
        // 引数なし → fuzzy 選択 UI。直前 script は先頭にピン留めされる。
        None => match rnr::prompt::select_script(&pkg.scripts, storage.last_run_command.as_deref())? {
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

    // volta があり mise が非 active なら volta run でラップする（rnr 独自）。
    let argv = wrap_volta(argv, cmd_exists("volta"), mise_active());

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