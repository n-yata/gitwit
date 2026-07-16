use crate::git::{
    build_side_by_side_rows, count_changed_lines, DiffFile, DiffHunk, FileStatus, SideCell,
};

/// エクスポート対象の1ファイル分のデータ。
pub struct ExportEntry {
    pub file: DiffFile,
    pub hunks: Vec<DiffHunk>,
}

impl ExportEntry {
    pub fn new(file: DiffFile, hunks: Vec<DiffHunk>) -> Self {
        Self { file, hunks }
    }
}

/// HTMLへの埋め込み前に、テキスト・属性値どちらでも安全な形へエスケープする。
/// diff本文・ファイルパスはリポジトリ内の任意の内容（信頼できない入力）のため、
/// 例外なくこの関数を通してから埋め込むこと。
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn status_badge(status: &FileStatus) -> (&'static str, &'static str) {
    match status {
        FileStatus::Added => ("A", "added"),
        FileStatus::Deleted => ("D", "deleted"),
        FileStatus::Modified => ("M", "modified"),
        FileStatus::Renamed { .. } => ("R", "renamed"),
    }
}

fn render_file_row(idx: usize, entry: &ExportEntry) -> String {
    let (badge_text, badge_class) = status_badge(&entry.file.status);
    let path = escape_html(&entry.file.path);
    let (added, deleted) = count_changed_lines(&entry.hunks);

    let old_path_html = if let FileStatus::Renamed { old_path } = &entry.file.status {
        format!(
            r#"<span class="meta">&larr; {}</span>"#,
            escape_html(old_path)
        )
    } else {
        String::new()
    };

    let binary_html = if entry.file.is_binary {
        r#"<span class="meta">(binary)</span>"#
    } else {
        ""
    };

    format!(
        r#"<div class="file-row" data-target="diff-{idx}" onclick="showDiff({idx})">
  <span class="badge badge-{badge_class}">{badge_text}</span>
  <span class="path">{path}</span>
  {binary_html}
  {old_path_html}
  <span class="lines"><span class="added">+{added}</span> <span class="deleted">-{deleted}</span></span>
</div>"#
    )
}

fn render_side_cell_html(cell: &SideCell<'_>) -> String {
    match cell {
        SideCell::Empty => r#"<div class="cell empty">&nbsp;</div>"#.to_string(),
        SideCell::Line(line) => {
            let (class, prefix) = match line.kind {
                crate::git::DiffLineKind::Added => ("added", "+"),
                crate::git::DiffLineKind::Deleted => ("deleted", "-"),
                crate::git::DiffLineKind::Context => ("context", " "),
            };
            format!(
                r#"<div class="cell {class}">{prefix}{content}</div>"#,
                class = class,
                prefix = prefix,
                content = escape_html(&line.content)
            )
        }
    }
}

fn render_diff_section(idx: usize, entry: &ExportEntry) -> String {
    if entry.file.is_binary {
        return format!(
            r#"<div class="diff-section" id="diff-{idx}">
  <p class="meta">バイナリファイルのため差分を表示できません</p>
</div>"#
        );
    }

    if entry.hunks.is_empty() {
        return format!(
            r#"<div class="diff-section" id="diff-{idx}">
  <p class="meta">差分なし</p>
</div>"#
        );
    }

    let mut hunks_html = String::new();
    for hunk in &entry.hunks {
        let rows = build_side_by_side_rows(&hunk.lines);
        let mut rows_html = String::new();
        for (left, right) in &rows {
            rows_html.push_str(&format!(
                r#"<div class="row">{left}{right}</div>"#,
                left = render_side_cell_html(left),
                right = render_side_cell_html(right)
            ));
        }
        hunks_html.push_str(&format!(
            r#"<div class="hunk-header">{header}</div><div class="hunk-body">{rows_html}</div>"#,
            header = escape_html(&hunk.header),
            rows_html = rows_html
        ));
    }

    format!(r#"<div class="diff-section" id="diff-{idx}">{hunks_html}</div>"#)
}

const STYLE: &str = r#"
body { font-family: "Segoe UI", Meiryo, sans-serif; margin: 0; display: flex; height: 100vh; color: #222; }
#file-list { width: 360px; overflow-y: auto; border-right: 1px solid #d2d2d2; flex-shrink: 0; }
#diff-view { flex: 1; overflow: auto; }
.file-row { padding: 6px 8px; cursor: pointer; border-bottom: 1px solid #eee; font-size: 12px; display: flex; align-items: center; gap: 6px; }
.file-row:hover { background: #f5f5f5; }
.file-row.active { background: #e8f0fe; }
.badge { color: #fff; border-radius: 3px; padding: 1px 4px; font-size: 11px; font-family: monospace; }
.badge-added { background: #28a745; }
.badge-deleted { background: #d1242f; }
.badge-modified { background: #0075ca; }
.badge-renamed { background: #6c757d; }
.path { font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.meta { color: #828282; font-size: 11px; }
.lines { margin-left: auto; font-family: monospace; font-size: 11px; white-space: nowrap; }
.added { color: #006400; }
.deleted { color: #960000; }
.diff-section { display: none; padding: 8px; }
.diff-section.active { display: block; }
.hunk-header { background: #dbeafe; color: #00468c; font-family: monospace; font-size: 12px; padding: 2px 6px; }
.hunk-body { display: flex; flex-direction: column; }
.row { display: grid; grid-template-columns: 1fr 1fr; }
.cell { font-family: monospace; font-size: 12px; padding: 1px 6px; white-space: pre; overflow-x: auto; }
.cell.added { background: #ddf4dc; color: #006400; }
.cell.deleted { background: #ffdcdc; color: #960000; }
.cell.context { background: #fafafa; }
.cell.empty { background: #f0f0f0; }
"#;

const SCRIPT: &str = r#"
function showDiff(idx) {
  document.querySelectorAll('.diff-section').forEach(function (el) { el.classList.remove('active'); });
  document.querySelectorAll('.file-row').forEach(function (el) { el.classList.remove('active'); });
  var target = document.getElementById('diff-' + idx);
  if (target) { target.classList.add('active'); }
  var rows = document.querySelectorAll('.file-row');
  if (rows[idx]) { rows[idx].classList.add('active'); }
}
if (document.querySelector('.file-row')) { showDiff(0); }
"#;

/// 変更ファイル一覧 + クリックで表示するside-by-side差分を1つの自己完結HTMLに組み立てる。
/// 外部リソース（CSS/JS/フォント等）への参照は一切含めない。
pub fn build_export_html(entries: &[ExportEntry]) -> String {
    let file_rows: String = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| render_file_row(idx, entry))
        .collect();

    let diff_sections: String = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| render_diff_section(idx, entry))
        .collect();

    format!(
        r#"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<title>差分エクスポート</title>
<style>{style}</style>
</head>
<body>
<div id="file-list">{file_rows}</div>
<div id="diff-view">{diff_sections}</div>
<script>{script}</script>
</body>
</html>"#,
        style = STYLE,
        file_rows = file_rows,
        diff_sections = diff_sections,
        script = SCRIPT
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff::DiffLine;
    use crate::git::DiffLineKind;

    fn added_entry(path: &str) -> ExportEntry {
        ExportEntry::new(
            DiffFile {
                path: path.to_string(),
                status: FileStatus::Added,
                is_binary: false,
            },
            vec![DiffHunk {
                header: "@@ -0,0 +1 @@".to_string(),
                lines: vec![DiffLine {
                    kind: DiffLineKind::Added,
                    content: "<script>alert(1)</script>".to_string(),
                }],
            }],
        )
    }

    #[test]
    fn escape_html_neutralizes_script_tags() {
        let escaped = escape_html("<script>alert(1)</script>");
        assert!(!escaped.contains("<script>"));
        assert_eq!(escaped, "&lt;script&gt;alert(1)&lt;/script&gt;");
    }

    #[test]
    fn build_export_html_escapes_diff_content() {
        let html = build_export_html(&[added_entry("evil.txt")]);
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    }

    #[test]
    fn build_export_html_includes_all_file_paths() {
        let html = build_export_html(&[added_entry("a.txt"), added_entry("b.txt")]);
        assert!(html.contains("a.txt"));
        assert!(html.contains("b.txt"));
    }

    #[test]
    fn build_export_html_shows_line_counts() {
        let html = build_export_html(&[added_entry("a.txt")]);
        assert!(html.contains(">+1<"));
        assert!(html.contains(">-0<"));
    }

    #[test]
    fn build_export_html_binary_file_hides_diff_content() {
        let entry = ExportEntry::new(
            DiffFile {
                path: "image.png".to_string(),
                status: FileStatus::Modified,
                is_binary: true,
            },
            Vec::new(),
        );
        let html = build_export_html(&[entry]);
        assert!(html.contains("(binary)"));
        assert!(html.contains("バイナリファイルのため差分を表示できません"));
    }

    #[test]
    fn build_export_html_renamed_binary_file_shows_both_old_path_and_binary_note() {
        let entry = ExportEntry::new(
            DiffFile {
                path: "assets/new.png".to_string(),
                status: FileStatus::Renamed {
                    old_path: "assets/old.png".to_string(),
                },
                is_binary: true,
            },
            Vec::new(),
        );
        let html = build_export_html(&[entry]);
        assert!(html.contains("assets/new.png"));
        assert!(html.contains("assets/old.png"));
        assert!(html.contains("(binary)"));
        assert!(html.contains("バイナリファイルのため差分を表示できません"));
    }
}
