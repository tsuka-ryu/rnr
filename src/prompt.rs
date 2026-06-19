use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use inquire::ui::{RenderConfig, Styled};
use inquire::{InquireError, Select};

/// command を切り詰める際に確保しておく固定の余白（列数）。
/// 本家 nr の `limitText(description, terminalColumns - 15)` に合わせた値で、
/// prefix・key・区切りぶんをざっくり見込んだもの。
const RESERVED_COLUMNS: usize = 15;
/// key と command の区切り。
const SEPARATOR: &str = " - ";

/// scripts 一覧から fuzzy 絞り込みで1つ選ばせる。
///
/// 表示は "key - command" 形式。長い command はターミナル幅に収まるよう
/// 切り詰めて末尾に `…` を付ける（本家 nr の挙動に合わせ、折り返しを防ぐ）。
/// `last`（直前に実行した script）が scripts に存在すれば先頭にピン留めする（本家挙動）。
/// 戻り値: 選ばれた script の key。ESC / Ctrl-C でキャンセルされたら Ok(None)。
/// （キャンセルは正常系として呼び出し側で exit 1、本当のエラーだけ Err で返す）
pub fn select_script(
    scripts: &BTreeMap<String, String>,
    last: Option<&str>,
) -> Result<Option<String>> {
    let columns = terminal_columns();

    // 表示ラベル一覧と「ラベル → key」の対応を作る。
    let mut labels: Vec<String> = Vec::with_capacity(scripts.len());
    let mut label_to_key: HashMap<String, String> = HashMap::with_capacity(scripts.len());
    for (key, command) in scripts {
        // 本家同様、固定の余白を差し引いた幅に command を収める。
        let command = limit_text(command, columns.saturating_sub(RESERVED_COLUMNS));
        let label = format!("{key}{SEPARATOR}{command}");
        label_to_key.insert(label.clone(), key.clone());
        labels.push(label);
    }

    // last が scripts にあれば、その行を先頭へ移動してピン留めする。
    if let Some(last) = last {
        let pinned = format!("{last}{SEPARATOR}");
        if let Some(pos) = labels.iter().position(|l| l.starts_with(&pinned)) {
            let item = labels.remove(pos);
            labels.insert(0, item);
        }
    }

    // スクロール上下インジケータを既定の `^`/`v` から `↑`/`↓` に差し替える。
    // ベースは default()（NO_COLOR 尊重）を使い、この 2 つだけ上書きする。
    let render_config = RenderConfig::default()
        .with_scroll_up_prefix(Styled::new("↑"))
        .with_scroll_down_prefix(Styled::new("↓"));

    match Select::new("script to run", labels)
        .with_render_config(render_config)
        .prompt()
    {
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

/// 現在のターミナル幅（列数）。TTY でない・取得失敗時は 80 にフォールバック（本家挙動）。
fn terminal_columns() -> usize {
    crossterm::terminal::size()
        .map(|(cols, _)| cols as usize)
        .unwrap_or(80)
}

/// `text` を表示幅 `max_width`（文字数）に収める。超える場合は切り詰めて末尾に `…`
/// を付ける（`…` も幅 1 に数える）。`max_width == 0` のときは空文字列。
fn limit_text(text: &str, max_width: usize) -> String {
    if text.chars().count() <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    let mut truncated: String = text.chars().take(max_width - 1).collect();
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_text_within_width() {
        assert_eq!(limit_text("vite build", 80), "vite build");
        assert_eq!(limit_text("abc", 3), "abc");
    }

    #[test]
    fn truncates_and_appends_ellipsis() {
        // max_width=5 なら 4 文字 + … で計 5 文字幅に収まる。
        assert_eq!(limit_text("abcdefg", 5), "abcd…");
        assert_eq!(limit_text("abcdefg", 5).chars().count(), 5);
    }

    #[test]
    fn zero_width_yields_empty() {
        assert_eq!(limit_text("anything", 0), "");
    }

    #[test]
    fn counts_by_chars_not_bytes() {
        // マルチバイトでもバイト境界で割らない。
        assert_eq!(limit_text("あいうえお", 3), "あい…");
    }
}