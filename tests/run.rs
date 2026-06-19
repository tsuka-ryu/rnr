// 統合テスト: lockfile + args → 期待する argv / Agent を検証する。
// lib crate (rnr) の公開 API だけを使う。

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rnr::command::build_run_args;
use rnr::detect::{Agent, detect};

// --- build_run_args（純粋関数。ファイル不要） ---

#[test]
fn build_pnpm_no_extra() {
    let argv = build_run_args(Agent::Pnpm, "build", &[]);
    assert_eq!(argv, vec!["pnpm", "run", "build"]);
}

#[test]
fn build_npm_with_extra() {
    let extra = vec!["--".to_string(), "--watch".to_string()];
    let argv = build_run_args(Agent::Npm, "test", &extra);
    assert_eq!(argv, vec!["npm", "run", "test", "--", "--watch"]);
}

#[test]
fn build_yarn() {
    let argv = build_run_args(Agent::Yarn, "dev", &[]);
    assert_eq!(argv, vec!["yarn", "run", "dev"]);
}

#[test]
fn build_bun() {
    let argv = build_run_args(Agent::Bun, "start", &[]);
    assert_eq!(argv, vec!["bun", "run", "start"]);
}

// --- detect（一時ディレクトリに lockfile を置いて検証） ---

// 衝突しないユニークな一時ディレクトリを作る。
// 標準ライブラリだけで、プロセス内カウンタ + ナノ秒 + pid から名前を作る。
fn unique_temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "rnr-test-{tag}-{pid}-{nanos}-{n}",
        pid = std::process::id(),
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

// 指定した lockfile 群を空ファイルとして置き、detect 結果を確認して後始末する。
fn detect_with_lockfiles(tag: &str, lockfiles: &[&str]) -> Agent {
    let dir = unique_temp_dir(tag);
    for f in lockfiles {
        fs::write(dir.join(f), "").unwrap();
    }
    let agent = detect(&dir);
    fs::remove_dir_all(&dir).unwrap();
    agent
}

#[test]
fn detect_pnpm() {
    assert_eq!(detect_with_lockfiles("pnpm", &["pnpm-lock.yaml"]), Agent::Pnpm);
}

#[test]
fn detect_yarn() {
    assert_eq!(detect_with_lockfiles("yarn", &["yarn.lock"]), Agent::Yarn);
}

#[test]
fn detect_bun_lockb() {
    assert_eq!(detect_with_lockfiles("bun-lockb", &["bun.lockb"]), Agent::Bun);
}

#[test]
fn detect_bun_lock() {
    assert_eq!(detect_with_lockfiles("bun-lock", &["bun.lock"]), Agent::Bun);
}

#[test]
fn detect_npm() {
    assert_eq!(detect_with_lockfiles("npm", &["package-lock.json"]), Agent::Npm);
}

#[test]
fn detect_fallback_to_npm_when_empty() {
    assert_eq!(detect_with_lockfiles("empty", &[]), Agent::Npm);
}

#[test]
fn detect_priority_pnpm_over_yarn() {
    // 複数 lockfile がある場合は優先順位どおり Pnpm が勝つ。
    assert_eq!(
        detect_with_lockfiles("priority", &["pnpm-lock.yaml", "yarn.lock"]),
        Agent::Pnpm
    );
}