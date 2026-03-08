use std::path::{Path, PathBuf};

use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedPasteBlob {
    pub file_path: String,
    pub char_count: usize,
}

pub fn persist_paste_blob(root: &Path, paste_id: &str, text: &str) -> AppResult<PersistedPasteBlob> {
    let relative_path = format!("pastes/{paste_id}.txt");
    let full_path = root.join(&relative_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, text)?;
    Ok(PersistedPasteBlob {
        file_path: relative_path,
        char_count: text.chars().count(),
    })
}

pub fn read_paste_blob(root: &Path, relative_path: &str) -> AppResult<String> {
    let full_path = root.join(relative_path);
    Ok(std::fs::read_to_string(full_path)?)
}

pub fn delete_paste_blob_file(root: &Path, relative_path: &str) -> AppResult<()> {
    let full_path = root.join(relative_path);
    match std::fs::remove_file(&full_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

pub fn expand_paste_refs_to_plain_text<F>(
    root: &Path,
    content: &str,
    resolve_path: &F,
) -> AppResult<String>
where
    F: Fn(&str) -> AppResult<String>,
{
    let content = expand_legacy_inline_pastes(content);
    transform_paste_refs(root, &content, resolve_path, |text, _count| text.to_string())
}

pub fn hydrate_paste_refs_to_legacy_markers<F>(
    root: &Path,
    content: &str,
    resolve_path: &F,
) -> AppResult<String>
where
    F: Fn(&str) -> AppResult<String>,
{
    transform_paste_refs(root, content, resolve_path, |text, count| {
        format!("<<paste:{count}>>{text}<</paste>>")
    })
}

pub fn expand_legacy_inline_pastes(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;
    while let Some(start) = remaining.find("<<paste:") {
        result.push_str(&remaining[..start]);
        let Some(marker_end) = remaining[start..].find(">>") else {
            result.push_str(&remaining[start..]);
            remaining = "";
            break;
        };
        let after_marker = start + marker_end + 2;
        let Some(close_pos) = remaining[after_marker..].find("<</paste>>") else {
            result.push_str(&remaining[start..]);
            remaining = "";
            break;
        };
        result.push_str(&remaining[after_marker..after_marker + close_pos]);
        remaining = &remaining[after_marker + close_pos + 10..];
    }
    result.push_str(remaining);
    result
}

pub fn externalize_legacy_inline_pastes<F>(content: &str, mut persist: F) -> AppResult<String>
where
    F: FnMut(&str, usize) -> AppResult<String>,
{
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;
    while let Some(start) = remaining.find("<<paste:") {
        result.push_str(&remaining[..start]);
        let Some(marker_end) = remaining[start..].find(">>") else {
            result.push_str(&remaining[start..]);
            remaining = "";
            break;
        };
        let count_text = &remaining[start + 8..start + marker_end];
        let after_marker = start + marker_end + 2;
        let Some(close_pos) = remaining[after_marker..].find("<</paste>>") else {
            result.push_str(&remaining[start..]);
            remaining = "";
            break;
        };
        let text = &remaining[after_marker..after_marker + close_pos];
        let count = count_text.parse::<usize>().unwrap_or_else(|_| text.chars().count());
        result.push_str(&persist(text, count)?);
        remaining = &remaining[after_marker + close_pos + 10..];
    }
    result.push_str(remaining);
    Ok(result)
}

fn transform_paste_refs<F, G>(
    root: &Path,
    content: &str,
    resolve_path: &F,
    render: G,
) -> AppResult<String>
where
    F: Fn(&str) -> AppResult<String>,
    G: Fn(&str, usize) -> String,
{
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    while let Some(start) = remaining.find("<<paste-ref:") {
        result.push_str(&remaining[..start]);
        let Some(marker_end) = remaining[start..].find(">>") else {
            result.push_str(&remaining[start..]);
            remaining = "";
            break;
        };
        let marker = &remaining[start + 12..start + marker_end];
        let mut parts = marker.splitn(2, ':');
        let paste_id = parts.next().unwrap_or_default();
        let count = parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or_default();
        if paste_id.is_empty() {
            result.push_str(&remaining[start..start + marker_end + 2]);
            remaining = &remaining[start + marker_end + 2..];
            continue;
        }
        let relative_path = resolve_path(paste_id)?;
        let text = read_paste_blob(root, &relative_path)?;
        result.push_str(&render(&text, count.max(text.chars().count())));
        remaining = &remaining[start + marker_end + 2..];
    }

    result.push_str(remaining);
    Ok(result)
}

pub fn unique_test_root(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[test]
    fn test_paste_storage_round_trip_and_expansion() {
        let root = unique_test_root("orion-chat-paste-test");
        std::fs::create_dir_all(&root).unwrap();

        let blob = persist_paste_blob(&root, "paste-1", "hello external paste").unwrap();
        assert_eq!(blob.char_count, 20);
        assert!(root.join(&blob.file_path).exists());

        let content = read_paste_blob(&root, &blob.file_path).unwrap();
        assert_eq!(content, "hello external paste");

        let expanded = expand_paste_refs_to_plain_text(
            &root,
            "before <<paste-ref:paste-1:20>> after",
            &|paste_id: &str| {
                if paste_id == "paste-1" {
                    Ok(blob.file_path.clone())
                } else {
                    Err(AppError::NotFound(paste_id.into()))
                }
            },
        )
        .unwrap();
        assert_eq!(expanded, "before hello external paste after");

        let hydrated = hydrate_paste_refs_to_legacy_markers(
            &root,
            "before <<paste-ref:paste-1:20>> after",
            &|paste_id: &str| {
                if paste_id == "paste-1" {
                    Ok(blob.file_path.clone())
                } else {
                    Err(AppError::NotFound(paste_id.into()))
                }
            },
        )
        .unwrap();
        assert_eq!(hydrated, "before <<paste:20>>hello external paste<</paste>> after");

        assert_eq!(
            expand_legacy_inline_pastes("A <<paste:4>>test<</paste>> B"),
            "A test B"
        );
        assert_eq!(
            externalize_legacy_inline_pastes(
                "A <<paste:4>>test<</paste>> B",
                |text, count| Ok(format!("<<paste-ref:demo:{count}>>:{text}")),
            )
            .unwrap(),
            "A <<paste-ref:demo:4>>:test B"
        );

        std::fs::remove_dir_all(root).unwrap();
    }
}
