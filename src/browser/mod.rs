pub mod page;

pub use page::{Link, Page};

use anyhow::{Context, Result};
use headless_chrome::{Browser as ChromeBrowser, LaunchOptions};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::time::Duration;
use std::sync::OnceLock;
use url::Url;
use regex::Regex;

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
        let mut links: Vec<Link> = document
            .select(&link_selector)
            .filter_map(|el| {
                let href = el.value().attr("href")?;
                let text = el.text().collect::<Vec<_>>().join(" ");
                let text = text.trim();

                // Skip empty, anchor-only, javascript, and citation-style links
                if text.is_empty()
                    || href == "#"
                    || href.starts_with("javascript:")
                    || is_citation_link(text, href)
                {
                    return None;
                }

                let full_url = resolve_url(&base_url, href);

                Some(Link {
                    text: text.to_string(),
                    url: full_url,
                })
            })
            .collect();

        // Extract images so they are discoverable via the Links panel (open to render)
        let images = extract_images(&document, &base_url);
        links.extend(images.iter().map(|img| Link {
            text: format!("[img] {}", img.label()),
            url: img.url.clone(),
        }));

        let is_wikipedia = is_wikipedia_url(&final_url);
        let content_lines = if is_wikipedia {
            extract_wikipedia_reading_lines(&self.client, &document, &base_url)
        } else {
            let reading_html = extract_main_html(&document, is_wikipedia).unwrap_or_else(|| html_content.clone());
            let content = html2text::from_read(reading_html.as_bytes(), 2000);
            let lines: Vec<String> = content
                .lines()
                .map(sanitize_text_line)
                .map(strip_inline_link_targets)
                .collect();
            let mut lines = prune_non_main_lines(lines, is_wikipedia);
            append_images_section(&mut lines, &images);
            lines
        };

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
                        "-------------------------------------------".to_string(),
                        "              PDF Document".to_string(),
                        "-------------------------------------------".to_string(),
                        format!("URL: {}", final_url),
                        "Press 'o' to open in system PDF viewer".to_string(),
                        "-------------------------------------------".to_string(),
                        "".to_string(),
                    ];
                    lines.extend(text.lines().map(|s| s.to_string()));
                    lines
                }
                Err(e) => {
                    vec![
                        "-------------------------------------------".to_string(),
                        "              PDF Document".to_string(),
                        "-------------------------------------------".to_string(),
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
            let image_bytes = response.bytes().context("Failed to read image bytes")?;
            let art = render_image_with_chafa(&image_bytes, 80, 32)
                .unwrap_or_else(|e| vec![format!("(image render error: {})", e)]);

            return Ok(Page {
                url: final_url.clone(),
                title: format!("Image ({})", image_type.to_uppercase()),
                content_lines: {
                    let mut lines = vec![
                        "-------------------------------------------".to_string(),
                        format!("              Image ({})", image_type.to_uppercase()),
                        "-------------------------------------------".to_string(),
                        "".to_string(),
                        format!("URL: {}", final_url),
                        "Press 'o' to open in system viewer".to_string(),
                        "".to_string(),
                    ];
                    lines.extend(art);
                    lines
                },
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
                    "-------------------------------------------".to_string(),
                    "              Binary File".to_string(),
                    "-------------------------------------------".to_string(),
                    "".to_string(),
                    format!("URL: {}", final_url),
                    format!("Type: {}", content_type),
                    "".to_string(),
                    "This is a binary file that cannot be".to_string(),
                    "displayed in a terminal browser.".to_string(),
                    "".to_string(),
                    "Options:".to_string(),
                    "  - Press 'o' to open/download".to_string(),
                    "  - Copy the URL and download manually".to_string(),
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
                    "-------------------------------------------".to_string(),
                    "              Binary File".to_string(),
                    "-------------------------------------------".to_string(),
                    "".to_string(),
                    format!("URL: {}", final_url),
                    "".to_string(),
                    "This file contains binary data that cannot".to_string(),
                    "be displayed in a terminal browser.".to_string(),
                    "".to_string(),
                    "Options:".to_string(),
                    "  - Press 'o' to open in system viewer".to_string(),
                    "  - Copy the URL and open in a browser".to_string(),
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
        let mut links: Vec<Link> = document
            .select(&link_selector)
            .filter_map(|el| {
                let href = el.value().attr("href")?;
                let text = el.text().collect::<Vec<_>>().join(" ");
                let text = text.trim();

                // Skip empty, anchor-only, javascript, and citation-style links
                if text.is_empty()
                    || href == "#"
                    || href.starts_with("javascript:")
                    || is_citation_link(text, href)
                {
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

        // Extract images so they are discoverable via the Links panel (open to render)
        let images = extract_images(&document, &base_url);
        links.extend(images.iter().map(|img| Link {
            text: format!("[img] {}", img.label()),
            url: img.url.clone(),
        }));

        let is_wikipedia = is_wikipedia_url(&final_url);
        let content_lines = if is_wikipedia {
            extract_wikipedia_reading_lines(&self.client, &document, &base_url)
        } else {
            let reading_html = extract_main_html(&document, is_wikipedia).unwrap_or_else(|| html_content.clone());
            let content = html2text::from_read(reading_html.as_bytes(), 2000);
            let lines: Vec<String> = content
                .lines()
                .map(sanitize_text_line)
                .map(strip_inline_link_targets)
                .collect();
            let mut lines = prune_non_main_lines(lines, is_wikipedia);
            append_images_section(&mut lines, &images);
            lines
        };

        Ok(Page {
            url: final_url,
            title,
            content_lines,
            raw_content: html_content,
            links,
        })
    }
}

#[derive(Debug, Clone)]
struct ImageRef {
    alt: String,
    url: String,
}

impl ImageRef {
    fn label(&self) -> String {
        let alt = self.alt.trim();
        if alt.is_empty() {
            filename_from_url(&self.url).unwrap_or_else(|| "image".to_string())
        } else {
            alt.to_string()
        }
    }
}

fn extract_images(document: &Html, base_url: &Url) -> Vec<ImageRef> {
    let selector = match Selector::parse("img") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();

    for el in document.select(&selector).take(200) {
        let Some(full_url) = best_image_url_from_el(&el, base_url) else {
            continue;
        };
        if !seen.insert(full_url.clone()) {
            continue;
        }
        let alt = el.value().attr("alt").unwrap_or("").trim().to_string();
        out.push(ImageRef { alt, url: full_url });
        if out.len() >= 50 {
            break;
        }
    }

    out
}

fn best_image_url_from_el(el: &scraper::ElementRef<'_>, base_url: &Url) -> Option<String> {
    let attrs = el.value();

    // Prefer srcset/data-srcset (often higher quality), then data-src, then src.
    if let Some(srcset) = attrs.attr("data-srcset").or_else(|| attrs.attr("srcset")) {
        if let Some(url) = parse_srcset_best_url(srcset) {
            let url = url.trim();
            if !url.is_empty() && !url.starts_with("data:") {
                return Some(resolve_url(base_url, url));
            }
        }
    }

    if let Some(src) = attrs
        .attr("data-src")
        .or_else(|| attrs.attr("data-original"))
        .or_else(|| attrs.attr("data-lazy-src"))
        .or_else(|| attrs.attr("src"))
    {
        let src = src.trim();
        if !src.is_empty() && !src.starts_with("data:") {
            return Some(resolve_url(base_url, src));
        }
    }

    None
}

fn parse_srcset_best_url(srcset: &str) -> Option<String> {
    // Pick the last entry, which is typically the largest image.
    // Format: "url1 1x, url2 2x" or "url 320w, url 640w"
    let last = srcset.split(',').map(str::trim).filter(|s| !s.is_empty()).last()?;
    let url = last.split_whitespace().next()?;
    Some(url.to_string())
}

fn append_images_section(lines: &mut Vec<String>, images: &[ImageRef]) {
    if images.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push("---".to_string());
    lines.push("## Images".to_string());
    lines.push(String::new());

    for img in images.iter().take(20) {
        let alt = sanitize_markdown_text(&img.label());
        let url = img.url.trim();
        lines.push(format!("![{}]({})", alt, url));
    }
}

fn sanitize_markdown_text(s: &str) -> String {
    // Keep it simple: avoid breaking the ![alt](url) structure.
    s.replace('[', "(").replace(']', ")").replace('\n', " ").replace('\r', " ")
}

fn sanitize_text_line(s: &str) -> String {
    // Replace control characters with spaces while preserving readable Unicode
    // (needed for box/block drawing characters used by inline image rendering).
    s.chars()
        .map(|c| {
            if c == '\t' || c == ' ' {
                return c;
            }
            if c.is_control() {
                return ' ';
            }
            c
        })
        .collect::<String>()
        .replace('\u{00A0}', " ")
}

fn filename_from_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let seg = parsed.path_segments()?.last()?;
    let seg = seg.trim();
    if seg.is_empty() {
        None
    } else {
        Some(seg.to_string())
    }
}

fn render_image_with_chafa(image_bytes: &[u8], max_w: u16, max_h: u16) -> Result<Vec<String>> {
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("azul-image-{}.bin", ts));

    {
        let mut f = File::create(&path).context("Failed to create temp image file")?;
        f.write_all(image_bytes)
            .context("Failed to write temp image file")?;
    }

    let run = |invert: bool| -> Result<Vec<String>> {
        let mut cmd = Command::new("chafa");
        cmd.args([
            "-f",
            "symbols",
            "-c",
            "full",
            "-O",
            "0",
            "-s",
            &format!("{}x{}", max_w, max_h),
        ]);
        if invert {
            cmd.arg("--invert");
        }
        cmd.arg(path.to_string_lossy().as_ref());

        let output = cmd.output().context("Failed to run chafa")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("chafa failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = stdout.lines().map(|l| l.trim_end().to_string()).collect();
        Ok(lines)
    };

    let mut lines = run(false)?;
    let has_ink = lines.iter().any(|l| l.chars().any(|c| !c.is_whitespace()));
    if !has_ink {
        // Some images (e.g., black logos on transparent background) render as blank.
        // Retry inverted to make them visible on dark terminals.
        lines = run(true)?;
    }

    // Best-effort cleanup
    let _ = std::fs::remove_file(&path);

    Ok(lines)
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

fn is_wikipedia_url(url: &str) -> bool {
    Url::parse(url)
        .ok()
        .and_then(|u| u.domain().map(|d| d.ends_with("wikipedia.org")))
        .unwrap_or(false)
}

/// Check if a link is a citation/reference that should be filtered from the links panel.
/// Matches: [1], [2], [a], [note 1], [citation needed], #cite_ref-*, #cite_note-*, etc.
fn is_citation_link(text: &str, href: &str) -> bool {
    let text = text.trim();

    // Skip internal citation anchors
    if href.contains("#cite_") || href.contains("#ref") || href.contains("#note") {
        return true;
    }

    // Must be bracket-wrapped to be a citation
    if !text.starts_with('[') || !text.ends_with(']') {
        return false;
    }

    let inner = &text[1..text.len() - 1].trim().to_lowercase();

    // [1], [2], [42], etc. - numbered citations
    if inner.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    // [a], [b], [aa], etc. - lettered citations
    if inner.chars().all(|c| c.is_ascii_lowercase()) && inner.len() <= 2 {
        return true;
    }

    // [note 1], [note 2], etc.
    if inner.starts_with("note ") {
        return true;
    }

    // [citation needed], [clarification needed], [when?], [who?], etc.
    let editorial_markers = [
        "citation needed",
        "clarification needed",
        "when?",
        "who?",
        "where?",
        "why?",
        "how?",
        "by whom?",
        "according to whom?",
        "which?",
        "what?",
        "dubious",
        "disputed",
        "unreliable source?",
        "failed verification",
        "better source needed",
    ];
    if editorial_markers.iter().any(|m| inner.contains(m)) {
        return true;
    }

    false
}

fn extract_main_html(document: &Html, is_wikipedia: bool) -> Option<String> {
    let selectors: &[&str] = if is_wikipedia {
        &[
            "#mw-content-text .mw-parser-output",
            "#mw-content-text",
            "#content",
            "main",
            "article",
        ]
    } else {
        &[
            "article",
            "main",
            "[role=main]",
            "#content",
            "#main",
            "#main-content",
            ".content",
            ".article",
        ]
    };

    let mut best: Option<scraper::ElementRef<'_>> = None;
    let mut best_len: usize = 0;

    for sel_str in selectors {
        let Ok(sel) = Selector::parse(sel_str) else {
            continue;
        };
        if let Some(el) = document.select(&sel).next() {
            let text_len: usize = el
                .text()
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .map(|t| t.len())
                .sum();
            if text_len > best_len {
                best = Some(el);
                best_len = text_len;
            }
        }
    }

    best.map(|el| el.html())
}

fn strip_inline_link_targets(line: String) -> String {
    static RE_PARENS_URL: OnceLock<Regex> = OnceLock::new();
    static RE_BRACKET_URL: OnceLock<Regex> = OnceLock::new();

    let re_parens = RE_PARENS_URL
        .get_or_init(|| Regex::new(r"\s*\(https?://[^\s)]+[\)]").expect("valid regex"));
    let re_bracket = RE_BRACKET_URL
        .get_or_init(|| Regex::new(r"\s*\[https?://[^\s\]]+\]").expect("valid regex"));

    let mut s = re_parens.replace_all(&line, "").to_string();
    s = re_bracket.replace_all(&s, "").to_string();
    s.replace("[edit]", "").trim_end().to_string()
}

fn prune_non_main_lines(lines: Vec<String>, is_wikipedia: bool) -> Vec<String> {
    static RE_FOOTNOTE_URL: OnceLock<Regex> = OnceLock::new();
    static RE_MD_REFDEF: OnceLock<Regex> = OnceLock::new();

    let re_footnote_url = RE_FOOTNOTE_URL
        .get_or_init(|| Regex::new(r"^\s*\[\d+\]\s+https?://\S+\s*$").expect("valid regex"));
    let re_md_refdef = RE_MD_REFDEF
        .get_or_init(|| Regex::new(r"^\s*\[[^\]]+\]:\s+https?://\S+\s*$").expect("valid regex"));

    let mut out = Vec::new();
    let mut prev_blank = true;

    for mut line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !prev_blank {
                out.push(String::new());
            }
            prev_blank = true;
            continue;
        }

        if trimmed.starts_with("![") {
            out.push(line);
            prev_blank = false;
            continue;
        }

        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            if !trimmed.contains(' ') {
                continue;
            }
        }

        if re_footnote_url.is_match(trimmed) || re_md_refdef.is_match(trimmed) {
            continue;
        }

        if is_wikipedia {
            // Wikipedia chrome that ruins reading flow.
            if trimmed == "From Wikipedia, the free encyclopedia"
                || trimmed.starts_with("Jump to navigation")
                || trimmed.starts_with("Jump to search")
                || trimmed == "Contents"
                || trimmed == "Languages"
                || trimmed == "Language"
                || trimmed == "Tools"
                || trimmed == "In other projects"
                || trimmed == "Appearance"
                || trimmed == "Navigation menu"
            {
                continue;
            }

            // Drop empty edit markers or leftover bracket junk
            if trimmed == "[edit]" {
                continue;
            }

            // Remove common metadata lines that are mostly non-reading.
            if trimmed.starts_with("Coordinates:") || trimmed.starts_with("Retrieved from") {
                continue;
            }
        }

        line = line.replace('\u{00A0}', " ");
        out.push(line.trim_end().to_string());
        prev_blank = false;
    }

    out
}

fn extract_wikipedia_reading_lines(client: &Client, document: &Html, base_url: &Url) -> Vec<String> {
    let title = Selector::parse("h1#firstHeading")
        .ok()
        .and_then(|sel| document.select(&sel).next())
        .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "Wikipedia".to_string());

    let container_sel = Selector::parse("#mw-content-text .mw-parser-output")
        .ok()
        .or_else(|| Selector::parse("#mw-content-text").ok());

    let Some(container_sel) = container_sel else {
        return Vec::new();
    };
    let Some(container) = document.select(&container_sel).next() else {
        return Vec::new();
    };

    let mut lines: Vec<String> = vec![format!("# {}", title), String::new()];

    // Lead/infobox image first (common on Wikipedia).
    let mut rendered_images: HashSet<String> = HashSet::new();
    let mut render_budget: usize = 12;

    if let Ok(sel) = Selector::parse("table.infobox img, figure.infobox img") {
        if let Some(img) = document.select(&sel).next() {
            if let Some(url) = best_image_url_from_el(&img, base_url) {
                append_inline_image_block(
                    client,
                    &mut lines,
                    &mut rendered_images,
                    &mut render_budget,
                    "Lead image",
                    &url,
                );
            }
        }
    }

    // Walk direct children and keep narrative blocks only.
    for child in container.children() {
        let Some(el) = scraper::ElementRef::wrap(child) else {
            continue;
        };
        let tag = el.value().name();

        // Skip non-reading containers.
        if tag == "table" || tag == "style" || tag == "script" {
            continue;
        }

        if tag == "div" {
            let class = el.value().attr("class").unwrap_or("");
            if class.contains("toc")
                || class.contains("navbox")
                || class.contains("vertical-navbox")
                || class.contains("metadata")
                || class.contains("ambox")
                || class.contains("mw-references-wrap")
            {
                continue;
            }

            // Thumb images embedded in the flow.
            if class.contains("thumb") {
                if let Ok(img_sel) = Selector::parse("img") {
                    if let Some(img) = el.select(&img_sel).next() {
                        if let Some(url) = best_image_url_from_el(&img, base_url) {
                            let alt = img.value().attr("alt").unwrap_or("Image").trim();
                            let label = if alt.is_empty() { "Image" } else { alt };
                            append_inline_image_block(
                                client,
                                &mut lines,
                                &mut rendered_images,
                                &mut render_budget,
                                label,
                                &url,
                            );
                        }
                    }
                }
                continue;
            }

            continue;
        }

        if tag == "p" {
            let raw = normalize_ws(&el.text().collect::<Vec<_>>().join(" "));
            let text = strip_wikipedia_inline_annotations(&raw);
            // Lower threshold to include shorter but meaningful paragraphs
            if text.len() >= 20 {
                lines.push(text);
                lines.push(String::new());
            }
            continue;
        }

        // Handle lists for better content extraction
        if tag == "ul" || tag == "ol" {
            // Skip navigation/metadata lists
            let class = el.value().attr("class").unwrap_or("");
            if class.contains("nav") || class.contains("menu") || class.contains("references") {
                continue;
            }

            if let Ok(li_sel) = Selector::parse("li") {
                for li in el.select(&li_sel).take(20) {
                    let raw = normalize_ws(&li.text().collect::<Vec<_>>().join(" "));
                    let text = strip_wikipedia_inline_annotations(&raw);
                    if text.len() >= 10 {
                        lines.push(format!("- {}", text));
                    }
                }
                if !lines.last().map(|l| l.is_empty()).unwrap_or(true) {
                    lines.push(String::new());
                }
            }
            continue;
        }

        // Definition lists (common in Wikipedia for key-value info)
        if tag == "dl" {
            if let Ok(dt_sel) = Selector::parse("dt") {
                if let Ok(dd_sel) = Selector::parse("dd") {
                    for dt in el.select(&dt_sel).take(10) {
                        let term = normalize_ws(&dt.text().collect::<Vec<_>>().join(" "));
                        let term = strip_wikipedia_inline_annotations(&term);
                        if !term.is_empty() {
                            lines.push(format!("**{}**", term));
                        }
                    }
                    for dd in el.select(&dd_sel).take(10) {
                        let def = normalize_ws(&dd.text().collect::<Vec<_>>().join(" "));
                        let def = strip_wikipedia_inline_annotations(&def);
                        if def.len() >= 10 {
                            lines.push(format!("  {}", def));
                        }
                    }
                    lines.push(String::new());
                }
            }
            continue;
        }

        if tag == "h2" || tag == "h3" || tag == "h4" {
            let raw = normalize_ws(&el.text().collect::<Vec<_>>().join(" "));
            let mut heading = strip_wikipedia_inline_annotations(&raw);
            heading = heading.replace("[edit]", "").trim().to_string();
            if heading.is_empty() {
                continue;
            }

            // Stop before non-narrative sections to keep the reading flow clean.
            if heading == "References"
                || heading == "External links"
                || heading == "See also"
                || heading == "Notes"
                || heading == "Further reading"
            {
                break;
            }

            let prefix = match tag {
                "h2" => "##",
                "h3" => "###",
                _ => "####",
            };
            lines.push(format!("{} {}", prefix, heading));
            lines.push(String::new());
            continue;
        }
    }

    // References section (styled via [n] markers)
    if let Ok(ref_sel) = Selector::parse("ol.references > li, .reflist ol > li") {
        let mut refs: Vec<String> = Vec::new();
        for (i, li) in document.select(&ref_sel).take(50).enumerate() {
            let text = normalize_ws(&li.text().collect::<Vec<_>>().join(" "));
            if text.len() < 10 {
                continue;
            }
            let cleaned = text.replace('↑', "").trim().to_string();
            refs.push(format!("- [{}] {}", i + 1, cleaned));
        }

        if !refs.is_empty() {
            lines.push("## References".to_string());
            lines.push(String::new());
            lines.extend(refs);
            lines.push(String::new());
        }
    }

    let lines: Vec<String> = lines
        .into_iter()
        .map(|s| sanitize_text_line(&s))
        .map(strip_inline_link_targets)
        .collect();
    prune_non_main_lines(lines, true)
}

fn strip_wikipedia_inline_annotations(text: &str) -> String {
    static RE_CITE_TS: OnceLock<Regex> = OnceLock::new();
    static RE_CITE_NUM: OnceLock<Regex> = OnceLock::new();
    static RE_CITE_NOTE: OnceLock<Regex> = OnceLock::new();
    static RE_CITE_WORD: OnceLock<Regex> = OnceLock::new();
    static RE_PUNCT: OnceLock<Regex> = OnceLock::new();
    static RE_PAREN_OPEN: OnceLock<Regex> = OnceLock::new();
    static RE_PAREN_CLOSE: OnceLock<Regex> = OnceLock::new();

    // Examples from Wikipedia/plaintext conversions:
    // "..., [ 15 ] and ..." -> remove "[ 15 ]"
    // "[ 16 ] : 7:50 ..."  -> remove citation + timestamp annotation
    // "[ note 4 ]"         -> remove note markers
    let re_cite_ts = RE_CITE_TS
        .get_or_init(|| Regex::new(r"\[\s*\d+\s*\]\s*:\s*\d{1,2}:\d{2}").expect("valid regex"));
    let re_cite_num = RE_CITE_NUM
        .get_or_init(|| Regex::new(r"\[\s*\d+\s*\]").expect("valid regex"));
    let re_cite_note = RE_CITE_NOTE
        .get_or_init(|| Regex::new(r"(?i)\[\s*note\s*\d+\s*\]").expect("valid regex"));
    let re_cite_word = RE_CITE_WORD
        .get_or_init(|| Regex::new(r"(?i)\[\s*(citation needed|clarification needed)\s*\]").expect("valid regex"));
    let re_punct = RE_PUNCT
        .get_or_init(|| Regex::new(r"\s+([,.;:!?])").expect("valid regex"));
    let re_paren_open = RE_PAREN_OPEN
        .get_or_init(|| Regex::new(r"\(\s+").expect("valid regex"));
    let re_paren_close = RE_PAREN_CLOSE
        .get_or_init(|| Regex::new(r"\s+\)").expect("valid regex"));

    let mut s = re_cite_ts.replace_all(text, "").to_string();
    s = re_cite_note.replace_all(&s, "").to_string();
    s = re_cite_word.replace_all(&s, "").to_string();
    s = re_cite_num.replace_all(&s, "").to_string();
    s = normalize_ws(&s);
    s = re_punct.replace_all(&s, "$1").to_string();
    s = re_paren_open.replace_all(&s, "(").to_string();
    s = re_paren_close.replace_all(&s, ")").to_string();
    s.trim().to_string()
}

fn append_inline_image_block(
    client: &Client,
    out: &mut Vec<String>,
    rendered: &mut HashSet<String>,
    budget: &mut usize,
    label: &str,
    url: &str,
) {
    if !rendered.insert(url.to_string()) {
        return;
    }

    let alt = sanitize_markdown_text(label);
    out.push(format!("![{}]({})", alt, url));

    if *budget == 0 {
        out.push(String::new());
        return;
    }

    if let Some(art) = try_render_inline_image(client, url, 72, 28) {
        *budget = budget.saturating_sub(1);
        out.push("```".to_string());
        out.extend(art);
        out.push("```".to_string());
    }
    out.push(String::new());
}

fn try_render_inline_image(client: &Client, url: &str, max_w: u16, max_h: u16) -> Option<Vec<String>> {
    let resp = client.get(url).send().ok()?;
    let bytes = resp.bytes().ok()?;
    render_image_with_chafa(&bytes, max_w, max_h).ok()
}

fn normalize_ws(s: &str) -> String {
    static RE_WS: OnceLock<Regex> = OnceLock::new();
    let re = RE_WS.get_or_init(|| Regex::new(r"\s+").expect("valid regex"));
    re.replace_all(s.trim(), " ").to_string()
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
    let output = Command::new("pdftotext")
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
