use crate::browser::Page;
use anyhow::{Context, Result};
use chrono::Utc;
use scraper::{Html, Selector};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct ScrapedLink {
    pub text: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ScrapedPage {
    pub url: String,
    pub title: String,
    pub headings: Vec<String>,
    pub links: Vec<ScrapedLink>,
    pub content: String,
}

/// Extract structured data from a page
pub fn scrape_page(page: &Page) -> ScrapedPage {
    let document = Html::parse_document(&page.raw_content);
    let heading_selector = Selector::parse("h1, h2, h3").unwrap();

    let headings = document
        .select(&heading_selector)
        .filter_map(|h| {
            let text = h.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .collect::<Vec<_>>();

    let links = page
        .links
        .iter()
        .map(|l| ScrapedLink {
            text: l.text.clone(),
            url: l.url.clone(),
        })
        .collect();

    ScrapedPage {
        url: page.url.clone(),
        title: page.title.clone(),
        headings,
        links,
        content: page.content_lines.join("\n"),
    }
}

/// Save scraped data to scrapes/<timestamp>-<slug>.json and return the path
pub fn save_scrape(page: &Page) -> Result<PathBuf> {
    let data = scrape_page(page);
    let dir = Path::new("scrapes");
    fs::create_dir_all(dir).context("Failed to create scrapes directory")?;

    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let slug = slugify(&data.title);
    let filename = format!("{}-{}.json", timestamp, slug);
    let path = dir.join(filename);

    let json = serde_json::to_string_pretty(&data).context("Failed to serialize scrape")?;
    fs::write(&path, json).context("Failed to write scrape file")?;

    Ok(path)
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "page".to_string()
    } else {
        trimmed.chars().take(48).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify(""), "page");
    }
}
