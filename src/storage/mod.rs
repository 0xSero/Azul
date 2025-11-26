//! Storage module for bookmarks and history persistence

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// Database connection wrapper
pub struct Database {
    conn: Connection,
}

/// Bookmark entry
#[derive(Debug, Clone)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// History entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visited_at: DateTime<Utc>,
}

impl Database {
    /// Open or create database at default location
    pub fn open() -> Result<Self> {
        let path = Self::default_path()?;
        Self::open_at(path)
    }

    /// Open or create database at specified path
    pub fn open_at(path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }

        let conn = Connection::open(&path)
            .context("Failed to open database")?;

        let db = Self { conn };
        db.init_schema()?;

        Ok(db)
    }

    /// Get default database path
    fn default_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .context("Could not find data directory")?;
        Ok(data_dir.join("azul-browse").join("azul.db"))
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS bookmarks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                tags TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                visited_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_bookmarks_url ON bookmarks(url);
            CREATE INDEX IF NOT EXISTS idx_history_url ON history(url);
            CREATE INDEX IF NOT EXISTS idx_history_visited ON history(visited_at);
            "#
        ).context("Failed to initialize schema")?;

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BOOKMARKS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Add a new bookmark (or update if URL exists)
    pub fn add_bookmark(&self, url: &str, title: &str, tags: &[String]) -> Result<Bookmark> {
        let tags_str = tags.join(",");

        self.conn.execute(
            r#"INSERT INTO bookmarks (url, title, tags)
               VALUES (?1, ?2, ?3)
               ON CONFLICT(url) DO UPDATE SET
                 title = ?2, tags = ?3, updated_at = CURRENT_TIMESTAMP"#,
            params![url, title, tags_str],
        ).context("Failed to add bookmark")?;

        let id = self.conn.last_insert_rowid();
        let now = Utc::now();

        Ok(Bookmark {
            id,
            url: url.to_string(),
            title: title.to_string(),
            tags: tags.to_vec(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Remove a bookmark by URL
    pub fn remove_bookmark(&self, url: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM bookmarks WHERE url = ?1",
            params![url],
        ).context("Failed to remove bookmark")?;

        Ok(())
    }

    /// Check if a URL is bookmarked
    pub fn is_bookmarked(&self, url: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bookmarks WHERE url = ?1",
            params![url],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Get a bookmark by URL
    pub fn get_bookmark(&self, url: &str) -> Result<Option<Bookmark>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, tags, created_at, updated_at FROM bookmarks WHERE url = ?1"
        )?;

        let result = stmt.query_row(params![url], |row| {
            let tags_str: Option<String> = row.get(3)?;
            let tags = tags_str
                .map(|s| s.split(',').filter(|t| !t.is_empty()).map(String::from).collect())
                .unwrap_or_default();

            let created_at_str: String = row.get(4)?;
            let updated_at_str: String = row.get(5)?;

            Ok(Bookmark {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                tags,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        });

        match result {
            Ok(bookmark) => Ok(Some(bookmark)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List bookmarks with optional search filter
    pub fn list_bookmarks(&self, search: Option<&str>, limit: usize) -> Result<Vec<Bookmark>> {
        let mut bookmarks = Vec::new();

        let mapper = |row: &rusqlite::Row| -> rusqlite::Result<Bookmark> {
            let tags_str: Option<String> = row.get(3)?;
            let tags = tags_str
                .map(|s| s.split(',').filter(|t| !t.is_empty()).map(String::from).collect())
                .unwrap_or_default();

            Ok(Bookmark {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                tags,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        };

        if let Some(s) = search {
            let search_val = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                r#"SELECT id, url, title, tags, created_at, updated_at FROM bookmarks
                   WHERE title LIKE ?1 OR url LIKE ?1 OR tags LIKE ?1
                   ORDER BY updated_at DESC LIMIT ?2"#,
            )?;
            let rows = stmt.query_map(params![search_val, limit as i64], mapper)?;
            for row in rows {
                bookmarks.push(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                r#"SELECT id, url, title, tags, created_at, updated_at FROM bookmarks
                   ORDER BY updated_at DESC LIMIT ?1"#,
            )?;
            let rows = stmt.query_map(params![limit as i64], mapper)?;
            for row in rows {
                bookmarks.push(row?);
            }
        }

        Ok(bookmarks)
    }

    /// Count total bookmarks
    pub fn count_bookmarks(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bookmarks",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HISTORY
    // ═══════════════════════════════════════════════════════════════════════════

    /// Add a history entry
    pub fn add_history(&self, url: &str, title: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO history (url, title) VALUES (?1, ?2)",
            params![url, title],
        ).context("Failed to add history")?;

        Ok(())
    }

    /// List history entries with optional search filter
    pub fn list_history(&self, search: Option<&str>, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut entries = Vec::new();

        let mapper = |row: &rusqlite::Row| -> rusqlite::Result<HistoryEntry> {
            Ok(HistoryEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                visited_at: Utc::now(),
            })
        };

        if let Some(s) = search {
            let search_val = format!("%{}%", s);
            let mut stmt = self.conn.prepare(
                r#"SELECT id, url, title, visited_at FROM history
                   WHERE title LIKE ?1 OR url LIKE ?1
                   ORDER BY visited_at DESC LIMIT ?2"#,
            )?;
            let rows = stmt.query_map(params![search_val, limit as i64], mapper)?;
            for row in rows {
                entries.push(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                r#"SELECT id, url, title, visited_at FROM history
                   ORDER BY visited_at DESC LIMIT ?1"#,
            )?;
            let rows = stmt.query_map(params![limit as i64], mapper)?;
            for row in rows {
                entries.push(row?);
            }
        }

        Ok(entries)
    }

    /// Clear all history
    pub fn clear_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM history", [])
            .context("Failed to clear history")?;
        Ok(())
    }

    /// Clear history older than specified days
    pub fn clear_history_older_than(&self, days: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM history WHERE visited_at < datetime('now', ?1)",
            params![format!("-{} days", days)],
        ).context("Failed to clear old history")?;
        Ok(())
    }

    /// Count total history entries
    pub fn count_history(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM history",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    // Return both so TempDir stays alive while Database is used
    fn test_db() -> (Database, TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open_at(path).unwrap();
        (db, dir)
    }

    #[test]
    fn test_add_and_get_bookmark() {
        let (db, _dir) = test_db();

        db.add_bookmark("https://example.com", "Example", &["test".to_string()]).unwrap();

        assert!(db.is_bookmarked("https://example.com").unwrap());

        let bookmark = db.get_bookmark("https://example.com").unwrap().unwrap();
        assert_eq!(bookmark.url, "https://example.com");
        assert_eq!(bookmark.title, "Example");
    }

    #[test]
    fn test_remove_bookmark() {
        let (db, _dir) = test_db();

        db.add_bookmark("https://example.com", "Example", &[]).unwrap();
        assert!(db.is_bookmarked("https://example.com").unwrap());

        db.remove_bookmark("https://example.com").unwrap();
        assert!(!db.is_bookmarked("https://example.com").unwrap());
    }

    #[test]
    fn test_add_history() {
        let (db, _dir) = test_db();

        db.add_history("https://example.com", "Example").unwrap();

        let entries = db.list_history(None, 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://example.com");
    }
}
