use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::browser::Page;

fn fixture_root() -> Option<PathBuf> {
    std::env::var("AZUL_FIXTURE_DIR").ok().map(PathBuf::from)
}

fn slug_for_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    let trimmed = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);

    let mut out = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    out
}

fn page_path(root: &Path, url: &str) -> PathBuf {
    root.join("pages")
        .join(format!("{}.json", slug_for_url(url)))
}

pub fn load_page_for_url(url: &str) -> Result<Option<Page>> {
    let root = match fixture_root() {
        Some(root) => root,
        None => return Ok(None),
    };

    let path = page_path(&root, url);
    if !path.exists() {
        anyhow::bail!(
            "fixture missing for URL {} (expected {})",
            url,
            path.display()
        );
    }

    let data = std::fs::read(&path)
        .with_context(|| format!("failed to read fixture {}", path.display()))?;
    let page: Page = serde_json::from_slice(&data)
        .with_context(|| format!("failed to parse fixture {}", path.display()))?;

    Ok(Some(page))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn returns_none_when_disabled() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("AZUL_FIXTURE_DIR");

        let result = load_page_for_url("https://fixture.local/article").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn loads_fixture_when_present() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let pages_dir = temp_dir.path().join("pages");
        std::fs::create_dir_all(&pages_dir).unwrap();

        let url = "https://fixture.local/article";
        let path = page_path(temp_dir.path(), url);
        let fixture = Page {
            url: url.to_string(),
            title: "Fixture Article".to_string(),
            content_lines: vec!["Line".to_string()],
            raw_content: "Raw".to_string(),
            links: vec![],
        };
        std::fs::write(&path, serde_json::to_vec(&fixture).unwrap()).unwrap();

        std::env::set_var("AZUL_FIXTURE_DIR", temp_dir.path());
        let loaded = load_page_for_url(url).unwrap().unwrap();
        std::env::remove_var("AZUL_FIXTURE_DIR");

        assert_eq!(loaded.title, "Fixture Article");
    }

    #[test]
    fn errors_when_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("pages")).unwrap();

        std::env::set_var("AZUL_FIXTURE_DIR", temp_dir.path());
        let result = load_page_for_url("https://fixture.local/missing");
        std::env::remove_var("AZUL_FIXTURE_DIR");

        assert!(result.is_err());
    }

    #[test]
    fn errors_when_unreadable() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let pages_dir = temp_dir.path().join("pages");
        std::fs::create_dir_all(&pages_dir).unwrap();

        let url = "https://fixture.local/unreadable";
        let path = page_path(temp_dir.path(), url);
        std::fs::create_dir_all(&path).unwrap();

        std::env::set_var("AZUL_FIXTURE_DIR", temp_dir.path());
        let result = load_page_for_url(url);
        std::env::remove_var("AZUL_FIXTURE_DIR");

        assert!(result.is_err());
    }

    #[test]
    fn errors_when_invalid_json() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let pages_dir = temp_dir.path().join("pages");
        std::fs::create_dir_all(&pages_dir).unwrap();

        let url = "https://fixture.local/invalid";
        let path = page_path(temp_dir.path(), url);
        std::fs::write(&path, b"not json").unwrap();

        std::env::set_var("AZUL_FIXTURE_DIR", temp_dir.path());
        let result = load_page_for_url(url);
        std::env::remove_var("AZUL_FIXTURE_DIR");

        assert!(result.is_err());
    }

    #[test]
    fn slug_for_url_strips_http_and_trailing_slash() {
        let slug = slug_for_url("http://example.com/path/");
        assert_eq!(slug, "example_com_path");
    }

    #[test]
    fn slug_for_url_keeps_no_scheme() {
        let slug = slug_for_url("example.com/path");
        assert_eq!(slug, "example_com_path");
    }
}
