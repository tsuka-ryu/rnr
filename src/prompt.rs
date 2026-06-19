use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use inquire::{InquireError, Select};

/// scripts 一覧から fuzzy 絞り込みで1つ選ばせる。
///
/// 表示は "key - command" 形式。`last`（直前に実行した script）が scripts に
/// 存在すれば先頭にピン留めする（本家挙動）。
/// 戻り値: 選ばれた script の key。ESC / Ctrl-C でキャンセルされたら Ok(None)。
/// （キャンセルは正常系として呼び出し側で exit 1、本当のエラーだけ Err で返す）
pub fn select_script(
    scripts: &BTreeMap<String, String>,
    last: Option<&str>,
) -> Result<Option<String>> {
    // 表示ラベル一覧と「ラベル → key」の対応を作る。
    let mut labels: Vec<String> = Vec::with_capacity(scripts.len());
    let mut label_to_key: HashMap<String, String> = HashMap::with_capacity(scripts.len());
    for (key, command) in scripts {
        let label = format!("{key} - {command}");
        label_to_key.insert(label.clone(), key.clone());
        labels.push(label);
    }

    // last が scripts にあれば、その行を先頭へ移動してピン留めする。
    if let Some(last) = last {
        let pinned = format!("{last} - ");
        if let Some(pos) = labels.iter().position(|l| l.starts_with(&pinned)) {
            let item = labels.remove(pos);
            labels.insert(0, item);
        }
    }

    match Select::new("script to run", labels).prompt() {
        Ok(label) => {
            // ラベルから元の key を引く。
            let key = label_to_key
                .get(&label)
                .cloned()
                .expect("選択されたラベルは必ず対応表にある");
            Ok(Some(key))
        }
        // ESC / Ctrl-C はキャンセル扱い（呼び出し側で静かに exit 1）。
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(e) => Err(e.into()),
    }
}