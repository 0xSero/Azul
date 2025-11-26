pub mod page;

pub use page::{Link, Page};

use anyhow::{Context, Result};
use headless_chrome::{Browser as ChromeBrowser, LaunchOptions};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use url::Url;

pub struct Browser {
    client: Client,
}

/// Render mode for fetching pages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Fast HTTP-only fetch (no JavaScript)
    Static,
    /// Full browser rendering with JavaScript
    JavaScript,
    /// Try static first, fall back to JS if page looks empty
    Auto,
}

impl Browser {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Azul-Browse/3.0")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { client })
    }

    /// Fetch a page with the specified render mode
    pub fn fetch_with_mode(&self, url: &str, mode: RenderMode) -> Result<Page> {
        match mode {
            RenderMode::Static => self.fetch(url),
            RenderMode::JavaScript => self.fetch_with_js(url),
            RenderMode::Auto => {
                // Try static first
                let page = self.fetch(url)?;

                // Check if page looks like it needs JavaScript
                if Self::needs_javascript(&page) {
                    // Retry with JavaScript rendering
                    self.fetch_with_js(url)
                } else {
                    Ok(page)
                }
            }
        }
    }

    /// Check if a page likely needs JavaScript to render properly
    fn needs_javascript(page: &Page) -> bool {
        // Check for signs that JS is needed:
        // 1. Very little text content
        let text_len: usize = page.content_lines.iter().map(|l| l.len()).sum();
        if text_len < 200 {
            return true;
        }

        // 2. Contains noscript warning
        let content = page.content_lines.join(" ").to_lowercase();
        if content.contains("javascript") && content.contains("enable") {
            return true;
        }
        if content.contains("noscript") {
            return true;
        }

        // 3. Common JS framework indicators in raw HTML with little content
        let raw_lower = page.raw_content.to_lowercase();
        let has_react = raw_lower.contains("react") || raw_lower.contains("__next");
        let has_vue = raw_lower.contains("vue") || raw_lower.contains("nuxt");
        let has_angular = raw_lower.contains("ng-app") || raw_lower.contains("angular");

        if (has_react || has_vue || has_angular) && text_len < 500 {
            return true;
        }

        false
    }

    /// Fetch a page using headless Chrome for JavaScript rendering
    pub fn fetch_with_js(&self, url: &str) -> Result<Page> {
        // Ensure URL has protocol
        let url_str = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        };

        let base_url = Url::parse(&url_str).context("Invalid URL")?;

        // Launch headless Chrome
        let launch_options = LaunchOptions {
            headless: true,
            sandbox: true,
            window_size: Some((1920, 1080)),
            ..Default::default()
        };

        let browser = ChromeBrowser::new(launch_options)
            .context("Failed to launch headless Chrome. Is Chrome/Chromium installed?")?;

        let tab = browser.new_tab().context("Failed to create browser tab")?;

        // Navigate to the URL and wait for page to load
        tab.navigate_to(&url_str)
            .context("Failed to navigate to URL")?;

        // Wait for network to be mostly idle (page loaded)
        tab.wait_until_navigated()
            .context("Page failed to load")?;

        // Give JS a moment to render
        std::thread::sleep(Duration::from_millis(500));

        // Get the final URL after any redirects
        let final_url = tab.get_url();

        // Get the rendered HTML content
        let html_content = tab
            .get_content()
            .context("Failed to get page content")?;

        // Parse the rendered HTML
        let document = Html::parse_document(&html_content);

        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.inner_html().trim().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        // Extract links with proper URL resolution
        let link_selector = Selector::parse("a[href]").unwrap();
        let links: Vec<Link> = document
            .select(&link_selector)
            .filter_map(|el| {
                let href = el.value().attr("href")?;
                let text = el.text().collect::<Vec<_>>().join(" ");
                let text = text.trim();

                if text.is_empty() || href == "#" || href.starts_with("javascript:") {
                    return None;
                }

                let full_url = resolve_url(&base_url, href);

                Some(Link {
                    text: text.to_string(),
                    url: full_url,
                })
            })
            .collect();

        // Convert HTML to readable text
        let content = html2text::from_read(html_content.as_bytes(), 100);
        let content_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        Ok(Page {
            url: final_url,
            title,
            content_lines,
            raw_content: html_content,
            links,
        })
    }

    /// Standard HTTP fetch (no JavaScript)
    pub fn fetch(&self, url: &str) -> Result<Page> {
        // Ensure URL has protocol
        let url_str = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        };

        let base_url = Url::parse(&url_str).context("Invalid URL")?;

        let response = self
            .client
            .get(&url_str)
            .send()
            .with_context(|| format!("Failed to fetch URL: {}", url_str))?;

        // Get the final URL after redirects
        let final_url = response.url().to_string();

        let html_content = response.text().context("Failed to read response body")?;

        let document = Html::parse_document(&html_content);

        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.inner_html().trim().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        // Extract links with proper URL resolution
        let link_selector = Selector::parse("a[href]").unwrap();
        let links: Vec<Link> = document
            .select(&link_selector)
            .filter_map(|el| {
                let href = el.value().attr("href")?;
                let text = el.text().collect::<Vec<_>>().join(" ");
                let text = text.trim();

                if text.is_empty() || href == "#" || href.starts_with("javascript:") {
                    return None;
                }

                // Resolve relative URLs
                let full_url = resolve_url(&base_url, href);

                Some(Link {
                    text: text.to_string(),
                    url: full_url,
                })
            })
            .collect();

        // Convert HTML to readable text
        let content = html2text::from_read(html_content.as_bytes(), 100);
        let content_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        Ok(Page {
            url: final_url,
            title,
            content_lines,
            raw_content: html_content,
            links,
        })
    }
}

impl Default for Browser {
    fn default() -> Self {
        Self::new().expect("Failed to create browser")
    }
}

/// Resolve a potentially relative URL against a base URL
fn resolve_url(base: &Url, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    if href.starts_with("//") {
        return format!("https:{}", href);
    }

    // Try to resolve against base URL
    match base.join(href) {
        Ok(resolved) => resolved.to_string(),
        Err(_) => href.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_creation() {
        let browser = Browser::new();
        assert!(browser.is_ok());
    }

    #[test]
    fn test_fetch_example_com() {
        let browser = Browser::new().unwrap();
        let page = browser.fetch("https://example.com");
        assert!(page.is_ok());

        let page = page.unwrap();
        assert_eq!(page.title, "Example Domain");
        assert!(!page.content_lines.is_empty());
    }

    #[test]
    fn test_resolve_url() {
        let base = Url::parse("https://example.com/path/page.html").unwrap();

        assert_eq!(
            resolve_url(&base, "https://other.com"),
            "https://other.com"
        );
        assert_eq!(resolve_url(&base, "//cdn.example.com"), "https://cdn.example.com");
        assert_eq!(
            resolve_url(&base, "/absolute/path"),
            "https://example.com/absolute/path"
        );
        assert_eq!(
            resolve_url(&base, "relative.html"),
            "https://example.com/path/relative.html"
        );
    }
}
