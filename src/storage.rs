use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// 最後に実行した script を覚えておくための永続データ。
///
/// `rnr -`（直前再実行）と、選択 UI での先頭ピン留めに使う。
/// 単なるキャッシュなので、読み込み失敗時はエラーにせず空として扱う。
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Storage {
    // Option なので未設定なら JSON に出力しない（skip）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_command: Option<String>,
}

/// 保存先のパス: <一時ディレクトリ>/rnr/_storage.json。
/// 本家は antfu-ni を使うが rnr は独自ディレクトリにして衝突を避ける。
fn storage_path() -> PathBuf {
    std::env::temp_dir().join("rnr").join("_storage.json")
}

/// storage を読み込む。ファイルが無い・壊れている場合は空の Storage を返す
/// （キャッシュなので壊れていても致命的ではない、という本家の方針に合わせる）。
pub fn load() -> Storage {
    let path = storage_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Storage::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// storage を保存する。保存ディレクトリが無ければ作る。
pub fn save(storage: &Storage) -> anyhow::Result<()> {
    let path = storage_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("storage ディレクトリの作成に失敗: {}", dir.display()))?;
    }
    let json = serde_json::to_string(storage)?;
    std::fs::write(&path, json)
        .with_context(|| format!("storage の書き込みに失敗: {}", path.display()))?;
    Ok(())
}