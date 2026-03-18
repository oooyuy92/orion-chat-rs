use crate::types::ToolError;
use std::path::{Path, PathBuf};

fn current_dir() -> Result<PathBuf, ToolError> {
    std::env::current_dir().map_err(|e| ToolError::Failed(format!("Cannot resolve current directory: {}", e)))
}

fn absolute_path(path: &str, base: Option<&Path>) -> Result<PathBuf, ToolError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else if let Some(base) = base {
        Ok(base.join(path))
    } else {
        Ok(current_dir()?.join(path))
    }
}

fn canonicalize_path(path: &Path, kind: &str) -> Result<PathBuf, ToolError> {
    std::fs::canonicalize(path)
        .map_err(|e| ToolError::Failed(format!("Cannot access {} {}: {}", kind, path.display(), e)))
}

fn canonicalize_roots(roots: &[String]) -> Result<Vec<PathBuf>, ToolError> {
    roots
        .iter()
        .map(|root| canonicalize_path(&absolute_path(root, None)?, "root"))
        .collect()
}

fn inside_any_root(path: &Path, roots: &[PathBuf]) -> bool {
    roots
        .iter()
        .any(|root| path == root || path.starts_with(root))
}

fn outside_roots_error(path: &Path) -> ToolError {
    ToolError::Failed(format!("Path '{}' is outside allowed roots", path.display()))
}

pub(crate) fn resolve_existing_path(path: &str, allowed_roots: &[String]) -> Result<PathBuf, ToolError> {
    if allowed_roots.is_empty() {
        return absolute_path(path, None);
    }

    let roots = canonicalize_roots(allowed_roots)?;
    let candidate = absolute_path(path, roots.first().map(PathBuf::as_path))?;
    let canonical = canonicalize_path(&candidate, "path")?;

    if inside_any_root(&canonical, &roots) {
        Ok(canonical)
    } else {
        Err(outside_roots_error(&candidate))
    }
}

pub(crate) fn resolve_write_path(path: &str, allowed_roots: &[String]) -> Result<PathBuf, ToolError> {
    if allowed_roots.is_empty() {
        return absolute_path(path, None);
    }

    let roots = canonicalize_roots(allowed_roots)?;
    let candidate = absolute_path(path, roots.first().map(PathBuf::as_path))?;
    let parent = candidate.parent().ok_or_else(|| {
        ToolError::Failed(format!("Cannot determine parent directory for {}", candidate.display()))
    })?;
    let canonical_parent = canonicalize_path(parent, "parent directory")?;

    if !inside_any_root(&canonical_parent, &roots) {
        return Err(outside_roots_error(&candidate));
    }

    if candidate.exists() {
        let canonical = canonicalize_path(&candidate, "path")?;
        if !inside_any_root(&canonical, &roots) {
            return Err(outside_roots_error(&candidate));
        }
    }

    Ok(candidate)
}

pub(crate) fn resolve_directory_path(
    path: Option<&str>,
    root: Option<&str>,
) -> Result<PathBuf, ToolError> {
    match root {
        Some(root) => {
            let canonical_root = canonicalize_path(&absolute_path(root, None)?, "root")?;
            let requested = path.unwrap_or(".");
            let candidate = absolute_path(requested, Some(&canonical_root))?;
            let canonical = canonicalize_path(&candidate, "path")?;

            if canonical == canonical_root || canonical.starts_with(&canonical_root) {
                Ok(canonical)
            } else {
                Err(outside_roots_error(&candidate))
            }
        }
        None => absolute_path(path.unwrap_or("."), None),
    }
}
