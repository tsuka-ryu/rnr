use std::collections::BTreeMap;
use std::path::Path;
use anyhow::Context;

use serde::Deserialize;

/// package.json のうち必要な部分だけを表す構造体。
///
/// `Deserialize` を derive すると serde が「JSON → この構造体」の変換コードを
/// 自動生成する。scripts 以外のフィールドは serde が自動で無視する。
#[derive(Debug, Deserialize)]
pub struct PackageJson {
    /// "scripts" オブジェクト。BTreeMap なのでキーが常にソート順に並ぶ
    /// （後で一覧表示するときの順序が安定する）。
    /// #[serde(default)] により scripts キーが無い package.json でも空マップになる。
    #[serde(default)]
    pub scripts: BTreeMap<String, String>,
}

/// `dir` 直下の package.json を読み、scripts を取り出す。
pub fn read(dir: &Path) -> anyhow::Result<PackageJson> {
    let path = dir.join("package.json");
    // ファイルをテキストとして読む。失敗時は with_context でどのパスかを添える。
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("package.json の読み込みに失敗: {}", path.display()))?;
    // テキスト(JSON) を PackageJson にデシリアライズする。
    let pkg: PackageJson = serde_json::from_str(&text)
        .with_context(|| format!("package.json のパースに失敗: {}", path.display()))?;
    Ok(pkg)
}
