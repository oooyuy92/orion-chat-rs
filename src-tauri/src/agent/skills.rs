use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub name: String,
    pub path: String,
    pub description: String,
    pub enabled: bool,
}

/// Scan markdown files in the configured directory as skills.
pub fn scan_skills_dir(dir: &str) -> AppResult<Vec<SkillInfo>> {
    let path = Path::new(dir);
    if !path.exists() || !path.is_dir() {
        return Ok(vec![]);
    }

    let mut skills = Vec::new();
    for entry in std::fs::read_dir(path).map_err(crate::error::AppError::Io)? {
        let entry = entry.map_err(crate::error::AppError::Io)?;
        let file_path = entry.path();
        if file_path.extension().is_some_and(|ext| ext == "md") {
            let name = file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let content = std::fs::read_to_string(&file_path).unwrap_or_default();
            let description = content.lines().next().unwrap_or("").to_string();
            skills.push(SkillInfo {
                name,
                path: file_path.to_string_lossy().to_string(),
                description,
                enabled: true,
            });
        }
    }

    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skills = scan_skills_dir(dir.path().to_str().unwrap()).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_scan_dir_with_skills() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("code-review.md"),
            "# Code Review Skill\nReview code",
        )
        .unwrap();
        fs::write(dir.path().join("not-a-skill.txt"), "ignored").unwrap();

        let skills = scan_skills_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "code-review");
    }

    #[test]
    fn test_scan_nonexistent_dir() {
        let skills = scan_skills_dir("/nonexistent/path").unwrap();
        assert!(skills.is_empty());
    }
}
