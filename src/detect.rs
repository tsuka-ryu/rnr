use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

/// `dir` 直下の lockfile からパッケージマネージャを検出する。
///
/// 優先順位順に lockfile を調べ、最初に見つかったものを返す。
/// どれも無ければ `Npm` にフォールバックする。
/// 探索はカレントディレクトリ直下のみ（親ディレクトリへの遡りはまだしない）。
pub fn detect(dir: &Path) -> Agent {
    if dir.join("pnpm-lock.yaml").exists() {
        Agent::Pnpm
    } else if dir.join("yarn.lock").exists() {
        Agent::Yarn
    } else if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
        Agent::Bun
    } else if dir.join("package-lock.json").exists() {
        Agent::Npm
    } else {
        Agent::Npm
    }
}