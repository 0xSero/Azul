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

        // Check Content-Type to handle binary files gracefully
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        // Also check URL extension as fallback (some servers lie about content-type)
        let url_lower = final_url.to_lowercase();
        let is_pdf = content_type.contains("application/pdf")
            || url_lower.ends_with(".pdf")
            || url_lower.contains(".pdf?");
        let is_image = content_type.contains("image/")
            || url_lower.ends_with(".png")
            || url_lower.ends_with(".jpg")
            || url_lower.ends_with(".jpeg")
            || url_lower.ends_with(".gif")
            || url_lower.ends_with(".webp")
            || url_lower.ends_with(".svg");

        // Handle PDF files - extract text using pdftotext
        if is_pdf {
            // Get PDF bytes
            let pdf_bytes = response.bytes().context("Failed to read PDF bytes")?;

            // Try to extract text using pdftotext
            let content_lines = match extract_pdf_text(&pdf_bytes) {
                Ok(text) => {
                    let mut lines: Vec<String> = vec![
                        "═══════════════════════════════════════════".to_string(),
                        "              PDF Document".to_string(),
                        "═══════════════════════════════════════════".to_string(),
                        format!("URL: {}", final_url),
                        "Press 'o' to open in system PDF viewer".to_string(),
                        "═══════════════════════════════════════════".to_string(),
                        "".to_string(),
                    ];
                    lines.extend(text.lines().map(|s| s.to_string()));
                    lines
                }
                Err(e) => {
                    vec![
                        "═══════════════════════════════════════════".to_string(),
                        "              PDF Document".to_string(),
                        "═══════════════════════════════════════════".to_string(),
                        "".to_string(),
                        format!("URL: {}", final_url),
                        "".to_string(),
                        format!("Failed to extract text: {}", e),
                        "".to_string(),
                        "Install poppler-utils for PDF text extraction:".to_string(),
                        "  sudo apt install poppler-utils".to_string(),
                        "".to_string(),
                        "Or press 'o' to open in system viewer".to_string(),
                        "".to_string(),
                    ]
                }
            };

            return Ok(Page {
                url: final_url.clone(),
                title: "PDF Document".to_string(),
                content_lines,
                raw_content: String::new(),
                links: vec![],
            });
        }

        if is_image {
            let image_type = if content_type.contains("image/") {
                content_type.split('/').nth(1).unwrap_or("unknown")
            } else {
                url_lower.split('.').last().unwrap_or("unknown")
            };
            return Ok(Page {
                url: final_url.clone(),
                title: format!("Image ({})", image_type.to_uppercase()),
                content_lines: vec![
                    "═══════════════════════════════════════════".to_string(),
                    format!("              Image ({})", image_type.to_uppercase()),
                    "═══════════════════════════════════════════".to_string(),
                    "".to_string(),
                    format!("URL: {}", final_url),
                    "".to_string(),
                    "This is an image file. Terminal browsers".to_string(),
                    "cannot display images directly.".to_string(),
                    "".to_string(),
                    "Options:".to_string(),
                    "  • Press 'o' to open in system viewer".to_string(),
                    "  • Copy the URL and open in a browser".to_string(),
                    "".to_string(),
                ],
                raw_content: String::new(),
                links: vec![],
            });
        }

        // For other binary types, show a warning
        if content_type.contains("application/octet-stream")
            || content_type.contains("application/zip")
            || content_type.contains("application/x-")
            || content_type.contains("video/")
            || content_type.contains("audio/")
        {
            return Ok(Page {
                url: final_url.clone(),
                title: "Binary File".to_string(),
                content_lines: vec![
                    "═══════════════════════════════════════════".to_string(),
                    "              Binary File".to_string(),
                    "═══════════════════════════════════════════".to_string(),
                    "".to_string(),
                    format!("URL: {}", final_url),
                    format!("Type: {}", content_type),
                    "".to_string(),
                    "This is a binary file that cannot be".to_string(),
                    "displayed in a terminal browser.".to_string(),
                    "".to_string(),
                    "Options:".to_string(),
                    "  • Press 'o' to open/download".to_string(),
                    "  • Copy the URL and download manually".to_string(),
                    "".to_string(),
                ],
                raw_content: String::new(),
                links: vec![],
            });
        }

        let html_content = response.text().context("Failed to read response body")?;

        // Check for binary content by looking at first bytes or high ratio of non-printable chars
        let first_bytes: Vec<u8> = html_content.bytes().take(16).collect();
        let is_likely_binary = html_content.starts_with("%PDF")
            || first_bytes.starts_with(&[0x89, b'P', b'N', b'G'])  // PNG
            || html_content.starts_with("GIF8")
            || first_bytes.starts_with(&[0xFF, 0xD8, 0xFF])        // JPEG
            || first_bytes.starts_with(&[b'P', b'K', 0x03, 0x04])  // ZIP
            || {
                // Check if first 1000 chars have high ratio of non-printable
                let sample: String = html_content.chars().take(1000).collect();
                let non_printable = sample.chars().filter(|c| {
                    !c.is_ascii_graphic() && !c.is_ascii_whitespace()
                }).count();
                non_printable > sample.len() / 4  // More than 25% non-printable
            };

        if is_likely_binary {
            return Ok(Page {
                url: final_url.clone(),
                title: "Binary File".to_string(),
                content_lines: vec![
                    "═══════════════════════════════════════════".to_string(),
                    "              Binary File".to_string(),
                    "═══════════════════════════════════════════".to_string(),
                    "".to_string(),
                    format!("URL: {}", final_url),
                    "".to_string(),
                    "This file contains binary data that cannot".to_string(),
                    "be displayed in a terminal browser.".to_string(),
                    "".to_string(),
                    "Options:".to_string(),
                    "  • Press 'o' to open in system viewer".to_string(),
                    "  • Copy the URL and open in a browser".to_string(),
                    "".to_string(),
                ],
                raw_content: String::new(),
                links: vec![],
            });
        }

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

        // Convert HTML to readable text and sanitize non-printable characters
        let content = html2text::from_read(html_content.as_bytes(), 100);
        let content_lines: Vec<String> = content
            .lines()
            .map(|s| {
                // Replace non-printable chars with spaces, keeping only safe characters
                s.chars()
                    .map(|c| {
                        if c.is_ascii_graphic() || c == ' ' || c == '\t' {
                            c
                        } else if c > '\u{00A0}' && (c.is_alphanumeric() || c.is_ascii_punctuation()) {
                            c
                        } else {
                            ' '
                        }
                    })
                    .collect::<String>()
            })
            .collect();

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

/// Extract text from PDF bytes using pdftotext (poppler-utils)
fn extract_pdf_text(pdf_bytes: &[u8]) -> Result<String> {
    use std::io::Write;
    use std::process::Command;

    // Create a temp file for the PDF
    let temp_dir = std::env::temp_dir();
    let pdf_path = temp_dir.join(format!("azul_pdf_{}.pdf", std::process::id()));

    // Write PDF to temp file
    let mut file = std::fs::File::create(&pdf_path)
        .context("Failed to create temp PDF file")?;
    file.write_all(pdf_bytes)
        .context("Failed to write PDF to temp file")?;
    drop(file);

    // Run pdftotext to extract text (- means stdout)
    let mut cmd = Command::new("pdftotext");
    
    // Windows-specific: check common installation paths if pdftotext is not in PATH
    if cfg!(target_os = "windows") {
        let paths = [
            "pdftotext", // Try PATH first
            r"poppler_bin\poppler-24.02.0\Library\bin\pdftotext.exe", // Local dev path
            r"C:\ProgramData\chocolatey\lib\poppler\tools\bin\pdftotext.exe",
            r"C:\ProgramData\chocolatey\bin\pdftotext.exe",
        ];
        
        for path in paths {
            if path == "pdftotext" || std::path::Path::new(path).exists() {
                cmd = Command::new(path);
                // If it's the first entry, we don't know if it exists yet, 
                // but Command::new will handle it. If it's a specific path, we checked.
                if path != "pdftotext" { break; }
            }
        }
    }

    let output = cmd
        .arg("-layout")  // Maintain layout
        .arg(&pdf_path)
        .arg("-")        // Output to stdout
        .output();

    // Clean up temp file
    let _ = std::fs::remove_file(&pdf_path);

    match output {
        Ok(out) => {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                if text.trim().is_empty() {
                    anyhow::bail!("PDF appears to be image-based or has no extractable text")
                }
                Ok(text)
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                anyhow::bail!("pdftotext failed: {}", stderr)
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::bail!("pdftotext not found - install poppler-utils")
            }
            anyhow::bail!("Failed to run pdftotext: {}", e)
        }
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
