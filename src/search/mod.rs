use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Search engine types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineType {
    DuckDuckGo,
    Google,
    Wikipedia,
    ArXiv,
    Scholar,
    PubMed,
    OpenLibrary,
    Exa, // AI-powered semantic search
}

impl EngineType {
    pub fn name(&self) -> &'static str {
        match self {
            EngineType::DuckDuckGo => "DuckDuckGo",
            EngineType::Google => "Google",
            EngineType::Wikipedia => "Wikipedia",
            EngineType::ArXiv => "arXiv",
            EngineType::Scholar => "Google Scholar",
            EngineType::PubMed => "PubMed",
            EngineType::OpenLibrary => "OpenLibrary",
            EngineType::Exa => "Exa",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EngineType::DuckDuckGo => "Privacy-first general search",
            EngineType::Google => "General web search",
            EngineType::Wikipedia => "General knowledge encyclopedia",
            EngineType::ArXiv => "Pre-prints and academic papers",
            EngineType::Scholar => "Scholarly articles and citations",
            EngineType::PubMed => "Biomedical literature and research",
            EngineType::OpenLibrary => "Books and manuscripts",
            EngineType::Exa => "AI-powered semantic search",
        }
    }

    pub fn prefix(&self) -> &'static str {
        match self {
            EngineType::DuckDuckGo => "d:",
            EngineType::Google => "g:",
            EngineType::Wikipedia => "w:",
            EngineType::ArXiv => "a:",
            EngineType::Scholar => "s:",
            EngineType::PubMed => "p:",
            EngineType::OpenLibrary => "ol:",
            EngineType::Exa => "e:",
        }
    }

    /// Get engine type from prefix
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix.to_lowercase().as_str() {
            "d:" => Some(EngineType::DuckDuckGo),
            "g:" => Some(EngineType::Google),
            "w:" => Some(EngineType::Wikipedia),
            "a:" => Some(EngineType::ArXiv),
            "s:" => Some(EngineType::Scholar),
            "p:" => Some(EngineType::PubMed),
            "ol:" => Some(EngineType::OpenLibrary),
            "e:" | "exa:" => Some(EngineType::Exa),
            _ => None,
        }
    }
}

/// A search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub description: String,
    pub engine: String,
}

/// Search response containing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub engine: EngineType,
    pub results: Vec<SearchResult>,
    pub total: usize,
}

/// Search engine trait
pub trait SearchEngine: Send + Sync {
    fn search(&self, query: &str) -> Result<SearchResponse>;
    fn engine_type(&self) -> EngineType;
}

/// HTTP client wrapper for search engines
pub struct SearchClient {
    client: reqwest::blocking::Client,
}

impl SearchClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    pub fn get(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .context("Failed to send request")?;

        response.text().context("Failed to read response body")
    }
}

impl Default for SearchClient {
    fn default() -> Self {
        Self::new().expect("Failed to create search client")
    }
}

// === DuckDuckGo Search ===
// Uses the Instant Answers API (more reliable than HTML scraping)

pub struct DuckDuckGoEngine {
    client: SearchClient,
}

impl DuckDuckGoEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: SearchClient::new()?,
        })
    }
}

impl Default for DuckDuckGoEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create DuckDuckGo engine")
    }
}

/// DuckDuckGo API response structures
#[derive(Debug, Deserialize)]
struct DDGResponse {
    #[serde(rename = "Abstract")]
    abstract_text: String,
    #[serde(rename = "AbstractText")]
    abstract_full: String,
    #[serde(rename = "AbstractSource")]
    abstract_source: String,
    #[serde(rename = "AbstractURL")]
    abstract_url: String,
    #[serde(rename = "Heading")]
    heading: String,
    #[serde(rename = "Answer")]
    answer: String,
    #[serde(rename = "Definition")]
    definition: String,
    #[serde(rename = "DefinitionURL")]
    definition_url: String,
    #[serde(rename = "RelatedTopics")]
    related_topics: Vec<DDGTopic>,
    #[serde(rename = "Results")]
    results: Vec<DDGResult>,
    #[serde(rename = "Redirect")]
    redirect: String,
}

#[derive(Debug, Deserialize)]
struct DDGTopic {
    #[serde(rename = "FirstURL")]
    first_url: Option<String>,
    #[serde(rename = "Text")]
    text: Option<String>,
    #[serde(rename = "Result")]
    result: Option<String>,
    #[serde(rename = "Topics")]
    topics: Option<Vec<DDGTopic>>,
}

#[derive(Debug, Deserialize)]
struct DDGResult {
    #[serde(rename = "FirstURL")]
    first_url: String,
    #[serde(rename = "Text")]
    text: String,
    #[serde(rename = "Result")]
    result: String,
}

impl SearchEngine for DuckDuckGoEngine {
    fn search(&self, query: &str) -> Result<SearchResponse> {
        let encoded = urlencoding::encode(query);
        // Use DuckDuckGo's Instant Answers API (JSON, no CAPTCHA)
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            encoded
        );

        let response_text = self.client.get(&url)?;
        let ddg_resp: DDGResponse = serde_json::from_str(&response_text)
            .context("Failed to parse DuckDuckGo response")?;

        let mut results = Vec::new();

        // Add main abstract if present
        if !ddg_resp.abstract_full.is_empty() && !ddg_resp.abstract_url.is_empty() {
            results.push(SearchResult {
                title: if ddg_resp.heading.is_empty() {
                    query.to_string()
                } else {
                    ddg_resp.heading.clone()
                },
                url: ddg_resp.abstract_url.clone(),
                description: ddg_resp.abstract_full.clone(),
                engine: "DuckDuckGo".to_string(),
            });
        }

        // Add instant answer if present
        if !ddg_resp.answer.is_empty() {
            results.push(SearchResult {
                title: format!("Instant Answer: {}", query),
                url: format!("https://duckduckgo.com/?q={}", encoded),
                description: ddg_resp.answer.clone(),
                engine: "DuckDuckGo".to_string(),
            });
        }

        // Add definition if present
        if !ddg_resp.definition.is_empty() && !ddg_resp.definition_url.is_empty() {
            results.push(SearchResult {
                title: format!("Definition: {}", query),
                url: ddg_resp.definition_url.clone(),
                description: ddg_resp.definition.clone(),
                engine: "DuckDuckGo".to_string(),
            });
        }

        // Add related topics
        for topic in &ddg_resp.related_topics {
            if let (Some(url), Some(text)) = (&topic.first_url, &topic.text) {
                if !url.is_empty() && !text.is_empty() {
                    let title = topic
                        .result
                        .as_ref()
                        .and_then(|r| extract_ddg_title(r))
                        .unwrap_or_else(|| text.clone());
                    results.push(SearchResult {
                        title,
                        url: url.clone(),
                        description: text.clone(),
                        engine: "DuckDuckGo".to_string(),
                    });
                }
            }

            // Handle nested topics
            if let Some(nested) = &topic.topics {
                for sub in nested {
                    if let (Some(url), Some(text)) = (&sub.first_url, &sub.text) {
                        if !url.is_empty() && !text.is_empty() {
                            let title = sub
                                .result
                                .as_ref()
                                .and_then(|r| extract_ddg_title(r))
                                .unwrap_or_else(|| text.clone());
                            results.push(SearchResult {
                                title,
                                url: url.clone(),
                                description: text.clone(),
                                engine: "DuckDuckGo".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Add direct results
        for result in &ddg_resp.results {
            if !result.first_url.is_empty() && !result.text.is_empty() {
                let title = extract_ddg_title(&result.result).unwrap_or_else(|| result.text.clone());
                results.push(SearchResult {
                    title,
                    url: result.first_url.clone(),
                    description: result.text.clone(),
                    engine: "DuckDuckGo".to_string(),
                });
            }
        }

        // Add redirect if present
        if !ddg_resp.redirect.is_empty() {
            results.push(SearchResult {
                title: format!("Redirect: {}", query),
                url: ddg_resp.redirect.clone(),
                description: format!("DuckDuckGo suggests this page for: {}", query),
                engine: "DuckDuckGo".to_string(),
            });
        }

        // If DDG returns no results, fall back to Wikipedia
        if results.is_empty() {
            let wiki = WikipediaEngine::new()?;
            let wiki_resp = wiki.search(query)?;

            // If Wikipedia has results, use them with a note
            if !wiki_resp.results.is_empty() {
                return Ok(SearchResponse {
                    query: query.to_string(),
                    engine: EngineType::DuckDuckGo, // Keep engine type for display
                    results: wiki_resp.results.into_iter().map(|mut r| {
                        r.engine = "Wikipedia (via DuckDuckGo fallback)".to_string();
                        r
                    }).collect(),
                    total: 0,
                });
            }
        }

        Ok(SearchResponse {
            query: query.to_string(),
            engine: EngineType::DuckDuckGo,
            results,
            total: 0,
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::DuckDuckGo
    }
}

/// Extract title from DDG HTML-formatted result string
/// DDG returns results like "<a href="...">Title</a>Description..."
fn extract_ddg_title(result: &str) -> Option<String> {
    if result.is_empty() {
        return None;
    }
    let start = result.find('>')?;
    let end = result.find("</a>")?;
    if start < end {
        Some(result[start + 1..end].to_string())
    } else {
        None
    }
}

// === Wikipedia Search ===

pub struct WikipediaEngine {
    client: SearchClient,
}

impl WikipediaEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: SearchClient::new()?,
        })
    }
}

impl Default for WikipediaEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create Wikipedia engine")
    }
}

#[derive(Debug, Deserialize)]
struct WikipediaResponse {
    query: Option<WikipediaQuery>,
}

#[derive(Debug, Deserialize)]
struct WikipediaQuery {
    search: Vec<WikipediaSearchResult>,
}

#[derive(Debug, Deserialize)]
struct WikipediaSearchResult {
    title: String,
    pageid: u64,
    snippet: String,
}

impl SearchEngine for WikipediaEngine {
    fn search(&self, query: &str) -> Result<SearchResponse> {
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&format=json&srlimit=20",
            encoded
        );

        let response_text = self.client.get(&url)?;
        let response: WikipediaResponse = serde_json::from_str(&response_text)
            .context("Failed to parse Wikipedia response")?;

        let results = response
            .query
            .map(|q| {
                q.search
                    .into_iter()
                    .map(|r| SearchResult {
                        title: r.title.clone(),
                        url: format!("https://en.wikipedia.org/wiki/{}", r.title.replace(' ', "_")),
                        description: html_to_text(&r.snippet),
                        engine: "Wikipedia".to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SearchResponse {
            query: query.to_string(),
            engine: EngineType::Wikipedia,
            results,
            total: 0,
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::Wikipedia
    }
}

// === arXiv Search ===

pub struct ArxivEngine {
    client: SearchClient,
}

impl ArxivEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: SearchClient::new()?,
        })
    }
}

impl Default for ArxivEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create arXiv engine")
    }
}

impl SearchEngine for ArxivEngine {
    fn search(&self, query: &str) -> Result<SearchResponse> {
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results=20",
            encoded
        );

        let response_text = self.client.get(&url)?;
        let results = parse_arxiv_atom(&response_text);

        Ok(SearchResponse {
            query: query.to_string(),
            engine: EngineType::ArXiv,
            results,
            total: 0,
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::ArXiv
    }
}

fn parse_arxiv_atom(xml: &str) -> Vec<SearchResult> {
    // Simple XML parsing for arXiv Atom feed
    let mut results = vec![];

    // Split by entry
    for entry in xml.split("<entry>").skip(1) {
        let end = entry.find("</entry>").unwrap_or(entry.len());
        let entry = &entry[..end];

        let title = extract_xml_content(entry, "title")
            .unwrap_or_default()
            .replace('\n', " ")
            .trim()
            .to_string();

        let summary = extract_xml_content(entry, "summary")
            .unwrap_or_default()
            .replace('\n', " ")
            .trim()
            .to_string();

        let url = entry
            .split("<id>")
            .nth(1)
            .and_then(|s| s.split("</id>").next())
            .unwrap_or_default()
            .trim()
            .to_string();

        if !title.is_empty() && !url.is_empty() {
            results.push(SearchResult {
                title,
                url,
                description: if summary.len() > 300 {
                    format!("{}...", &summary[..300])
                } else {
                    summary
                },
                engine: "arXiv".to_string(),
            });
        }
    }

    results
}

fn extract_xml_content(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}", tag);
    let end_tag = format!("</{}>", tag);

    let start = xml.find(&start_tag)?;
    let content_start = xml[start..].find('>')? + start + 1;
    let end = xml[content_start..].find(&end_tag)? + content_start;

    Some(xml[content_start..end].to_string())
}

// === Google Scholar Search ===

pub struct ScholarEngine {
    client: SearchClient,
}

impl ScholarEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: SearchClient::new()?,
        })
    }
}

impl Default for ScholarEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create Scholar engine")
    }
}

impl SearchEngine for ScholarEngine {
    fn search(&self, query: &str) -> Result<SearchResponse> {
        let encoded = urlencoding::encode(query);
        let url = format!("https://scholar.google.com/scholar?q={}&hl=en", encoded);

        let html = self.client.get(&url)?;
        let results = parse_scholar_html(&html);

        Ok(SearchResponse {
            query: query.to_string(),
            engine: EngineType::Scholar,
            results,
            total: 0,
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::Scholar
    }
}

fn parse_scholar_html(html: &str) -> Vec<SearchResult> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let mut results = vec![];

    // Google Scholar result structure - use ok() to avoid panics
    let result_selector = match Selector::parse(".gs_r.gs_or.gs_scl") {
        Ok(s) => s,
        Err(_) => return results,
    };
    let title_selector = match Selector::parse(".gs_rt a") {
        Ok(s) => s,
        Err(_) => return results,
    };
    let snippet_selector = match Selector::parse(".gs_rs") {
        Ok(s) => s,
        Err(_) => return results,
    };

    for element in document.select(&result_selector) {
        let title = element
            .select(&title_selector)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();

        let url = element
            .select(&title_selector)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default()
            .to_string();

        let description = element
            .select(&snippet_selector)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();

        if !title.is_empty() {
            results.push(SearchResult {
                title: title.trim().to_string(),
                url: url.trim().to_string(),
                description: description.trim().to_string(),
                engine: "Google Scholar".to_string(),
            });
        }
    }

    results
}

// === PubMed Search ===

pub struct PubMedEngine {
    client: SearchClient,
}

impl PubMedEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: SearchClient::new()?,
        })
    }
}

impl Default for PubMedEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create PubMed engine")
    }
}

#[derive(Debug, Deserialize)]
struct PubMedSearchResponse {
    esearchresult: PubMedSearchResult,
}

#[derive(Debug, Deserialize)]
struct PubMedSearchResult {
    idlist: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PubMedSummaryResponse {
    result: std::collections::HashMap<String, serde_json::Value>,
}

impl SearchEngine for PubMedEngine {
    fn search(&self, query: &str) -> Result<SearchResponse> {
        let encoded = urlencoding::encode(query);

        // First, search for IDs
        let search_url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term={}&retmode=json&retmax=20",
            encoded
        );

        let search_response_text = self.client.get(&search_url)?;
        let search_response: PubMedSearchResponse = serde_json::from_str(&search_response_text)
            .context("Failed to parse PubMed search response")?;

        if search_response.esearchresult.idlist.is_empty() {
            return Ok(SearchResponse {
                query: query.to_string(),
                engine: EngineType::PubMed,
                results: vec![],
                total: 0,
            });
        }

        // Then fetch summaries
        let ids = search_response.esearchresult.idlist.join(",");
        let summary_url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={}&retmode=json",
            ids
        );

        let summary_response_text = self.client.get(&summary_url)?;
        let summary_response: PubMedSummaryResponse = serde_json::from_str(&summary_response_text)
            .context("Failed to parse PubMed summary response")?;

        let mut results = vec![];
        for id in &search_response.esearchresult.idlist {
            if let Some(article) = summary_response.result.get(id) {
                let title = article
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                let source = article
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let pubdate = article
                    .get("pubdate")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if !title.is_empty() {
                    results.push(SearchResult {
                        title,
                        url: format!("https://pubmed.ncbi.nlm.nih.gov/{}/", id),
                        description: format!("{} - {}", source, pubdate),
                        engine: "PubMed".to_string(),
                    });
                }
            }
        }

        Ok(SearchResponse {
            query: query.to_string(),
            engine: EngineType::PubMed,
            results,
            total: 0,
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::PubMed
    }
}

// === OpenLibrary Search ===

pub struct OpenLibraryEngine {
    client: SearchClient,
}

impl OpenLibraryEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: SearchClient::new()?,
        })
    }
}

impl Default for OpenLibraryEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create OpenLibrary engine")
    }
}

#[derive(Debug, Deserialize)]
struct OpenLibraryResponse {
    docs: Vec<OpenLibraryDoc>,
}

#[derive(Debug, Deserialize)]
struct OpenLibraryDoc {
    title: String,
    key: String,
    author_name: Option<Vec<String>>,
    first_publish_year: Option<i32>,
}

impl SearchEngine for OpenLibraryEngine {
    fn search(&self, query: &str) -> Result<SearchResponse> {
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://openlibrary.org/search.json?q={}&limit=20",
            encoded
        );

        let response_text = self.client.get(&url)?;
        let response: OpenLibraryResponse = serde_json::from_str(&response_text)
            .context("Failed to parse OpenLibrary response")?;

        let results = response
            .docs
            .into_iter()
            .map(|doc| {
                let authors = doc
                    .author_name
                    .map(|a| a.join(", "))
                    .unwrap_or_else(|| "Unknown author".to_string());

                let year = doc
                    .first_publish_year
                    .map(|y| format!(" ({})", y))
                    .unwrap_or_default();

                SearchResult {
                    title: doc.title,
                    url: format!("https://openlibrary.org{}", doc.key),
                    description: format!("{}{}", authors, year),
                    engine: "OpenLibrary".to_string(),
                }
            })
            .collect();

        Ok(SearchResponse {
            query: query.to_string(),
            engine: EngineType::OpenLibrary,
            results,
            total: 0,
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::OpenLibrary
    }
}

// === Exa Search ===
// AI-powered semantic search with highlights and summaries

pub struct ExaEngine {
    client: SearchClient,
    api_key: String,
}

impl ExaEngine {
    pub fn new(api_key: String) -> Result<Self> {
        Ok(Self {
            client: SearchClient::new()?,
            api_key,
        })
    }

    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        let api_key = config.get_exa_api_key()?;
        if api_key.is_empty() {
            return None;
        }
        Self::new(api_key).ok()
    }
}

#[derive(Debug, Deserialize)]
struct ExaResponse {
    results: Option<Vec<ExaResult>>,
    #[serde(rename = "autopromptString")]
    autoprompt_string: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExaResult {
    title: Option<String>,
    url: String,
    #[serde(rename = "publishedDate")]
    published_date: Option<String>,
    author: Option<String>,
    text: Option<String>,
    highlights: Option<Vec<String>>,
    #[serde(rename = "highlightScores")]
    highlight_scores: Option<Vec<f64>>,
    summary: Option<String>,
}

impl SearchEngine for ExaEngine {
    fn search(&self, query: &str) -> Result<SearchResponse> {
        let url = "https://api.exa.ai/search";

        // Build request body for Exa API
        let body = serde_json::json!({
            "query": query,
            "type": "neural",
            "useAutoprompt": true,
            "numResults": 10,
            "contents": {
                "text": { "maxCharacters": 500 },
                "highlights": { "numSentences": 2 },
                "summary": { "query": query }
            }
        });

        let response = reqwest::blocking::Client::new()
            .post(url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .json(&body)
            .timeout(Duration::from_secs(30))
            .send()
            .context("Failed to send Exa request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            anyhow::bail!("Exa API error {}: {}", status, text);
        }

        let exa_resp: ExaResponse = response.json()
            .context("Failed to parse Exa response")?;

        let results = exa_resp
            .results
            .unwrap_or_default()
            .into_iter()
            .map(|r| {
                // Build description from highlights, summary, or text
                let description = if let Some(highlights) = &r.highlights {
                    if !highlights.is_empty() {
                        highlights.join(" ... ")
                    } else if let Some(summary) = &r.summary {
                        summary.clone()
                    } else {
                        r.text.clone().unwrap_or_default()
                    }
                } else if let Some(summary) = &r.summary {
                    summary.clone()
                } else {
                    r.text.clone().unwrap_or_default()
                };

                // Truncate description if too long
                let description = if description.chars().count() > 300 {
                    format!("{}...", description.chars().take(300).collect::<String>())
                } else {
                    description
                };

                SearchResult {
                    title: r.title.unwrap_or_else(|| "Untitled".to_string()),
                    url: r.url,
                    description,
                    engine: "Exa".to_string(),
                }
            })
            .collect();

        Ok(SearchResponse {
            query: query.to_string(),
            engine: EngineType::Exa,
            results,
            total: 0,
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::Exa
    }
}

// === Search Manager ===

/// Manages multiple search engines
pub struct SearchManager {
    engines: std::collections::HashMap<EngineType, Box<dyn SearchEngine>>,
    selected: EngineType,
}

impl SearchManager {
    pub fn new() -> Result<Self> {
        let mut engines: std::collections::HashMap<EngineType, Box<dyn SearchEngine>> =
            std::collections::HashMap::new();

        engines.insert(EngineType::DuckDuckGo, Box::new(DuckDuckGoEngine::new()?));
        engines.insert(EngineType::Wikipedia, Box::new(WikipediaEngine::new()?));
        engines.insert(EngineType::ArXiv, Box::new(ArxivEngine::new()?));
        engines.insert(EngineType::Scholar, Box::new(ScholarEngine::new()?));
        engines.insert(EngineType::PubMed, Box::new(PubMedEngine::new()?));
        engines.insert(EngineType::OpenLibrary, Box::new(OpenLibraryEngine::new()?));

        // Try to add Exa from environment
        if let Ok(api_key) = std::env::var("EXA_API_KEY") {
            if !api_key.is_empty() {
                if let Ok(exa) = ExaEngine::new(api_key) {
                    engines.insert(EngineType::Exa, Box::new(exa));
                }
            }
        }

        Ok(Self {
            engines,
            selected: EngineType::DuckDuckGo,
        })
    }

    /// Create SearchManager with config (includes Exa if configured)
    pub fn with_config(config: &crate::config::Config) -> Result<Self> {
        let mut manager = Self::new()?;

        // Add Exa if configured
        if let Some(exa) = ExaEngine::from_config(config) {
            manager.engines.insert(EngineType::Exa, Box::new(exa));
        }

        Ok(manager)
    }

    /// Check if Exa is available
    pub fn has_exa(&self) -> bool {
        self.engines.contains_key(&EngineType::Exa)
    }

    pub fn set_engine(&mut self, engine: EngineType) -> bool {
        if self.engines.contains_key(&engine) {
            self.selected = engine;
            true
        } else {
            false
        }
    }

    pub fn selected(&self) -> EngineType {
        self.selected
    }

    pub fn search(&self, query: &str) -> Result<SearchResponse> {
        let engine = self
            .engines
            .get(&self.selected)
            .context("Selected engine not found")?;
        engine.search(query)
    }

    pub fn search_with(&self, engine: EngineType, query: &str) -> Result<SearchResponse> {
        let engine = self.engines.get(&engine).context("Engine not found")?;
        engine.search(query)
    }

    pub fn available_engines(&self) -> Vec<EngineType> {
        self.engines.keys().cloned().collect()
    }

    /// Search all engines and return limited results per engine
    /// This is the multi-engine aggregated search
    pub fn search_all_with_limit(&self, query: &str, limit_per_engine: usize) -> AggregatedSearchResponse {
        let mut all_results = Vec::new();
        let mut engine_results: std::collections::HashMap<EngineType, Vec<SearchResult>> =
            std::collections::HashMap::new();

        // Define the order of engines for display (Exa first if available)
        let engine_order = [
            EngineType::Exa,          // AI semantic search (if configured)
            EngineType::Wikipedia,
            EngineType::ArXiv,
            EngineType::Scholar,
            EngineType::DuckDuckGo,
            EngineType::PubMed,
            EngineType::OpenLibrary,
        ];

        for engine_type in &engine_order {
            if let Some(engine) = self.engines.get(engine_type) {
                if let Ok(response) = engine.search(query) {
                    let limited: Vec<SearchResult> = response
                        .results
                        .into_iter()
                        .take(limit_per_engine)
                        .collect();
                    engine_results.insert(*engine_type, limited);
                }
            }
        }

        // Collect results in engine order
        for engine_type in &engine_order {
            if let Some(results) = engine_results.get(engine_type) {
                all_results.extend(results.iter().cloned());
            }
        }

        AggregatedSearchResponse {
            query: query.to_string(),
            results: all_results,
            results_by_engine: engine_results,
        }
    }

    /// Convenience method: search all engines with 5 results each (default aggregated search)
    pub fn search_aggregated(&self, query: &str) -> AggregatedSearchResponse {
        self.search_all_with_limit(query, 5)
    }
}

/// Aggregated search response from multiple engines
#[derive(Debug, Clone)]
pub struct AggregatedSearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub results_by_engine: std::collections::HashMap<EngineType, Vec<SearchResult>>,
}

impl AggregatedSearchResponse {
    /// Convert to a display Page with engine grouping
    pub fn to_page(&self) -> crate::browser::Page {
        self.to_page_with_summary(None)
    }

    /// Convert to a display Page with optional AI summary at the top
    pub fn to_page_with_summary(&self, ai_summary: Option<String>) -> crate::browser::Page {
        use crate::browser::{Link, Page};

        let mut lines = vec![
            format!("# Multi-Engine Search: \"{}\"", self.query),
            String::new(),
        ];

        // Add AI summary at top if available
        if let Some(summary) = ai_summary {
            lines.push("## AI Summary".to_string());
            lines.push(String::new());
            lines.push(format!("> {}", summary));
            lines.push(String::new());
            lines.push("---".to_string());
            lines.push(String::new());
        }

        lines.push(format!("Found {} results across {} engines",
                self.results.len(),
                self.results_by_engine.len()));
        lines.push(String::new());
        lines.push("---".to_string());
        lines.push(String::new());

        let mut links = Vec::new();

        // Display results grouped by engine (Exa first if present)
        let engine_order = [
            EngineType::Exa,
            EngineType::Wikipedia,
            EngineType::ArXiv,
            EngineType::Scholar,
            EngineType::DuckDuckGo,
            EngineType::PubMed,
            EngineType::OpenLibrary,
        ];

        for engine_type in &engine_order {
            if let Some(results) = self.results_by_engine.get(engine_type) {
                if results.is_empty() {
                    continue;
                }

                lines.push(format!("## {} ({})", engine_type.name(), results.len()));
                lines.push(String::new());

                for (idx, result) in results.iter().enumerate() {
                    lines.push(format!("{}. **{}**", idx + 1, result.title));

                    if !result.description.is_empty() {
                        // Truncate long descriptions (safely handle UTF-8)
                        let desc = if result.description.chars().count() > 200 {
                            let truncated: String = result.description.chars().take(200).collect();
                            format!("{}...", truncated)
                        } else {
                            result.description.clone()
                        };
                        lines.push(format!("   > {}", desc));
                    }

                    lines.push(format!("   -- {}", result.url));
                    lines.push(String::new());

                    links.push(Link {
                        text: format!("[{}] {}", engine_type.name(), result.title),
                        url: result.url.clone(),
                    });
                }
            }
        }

        if self.results.is_empty() {
            lines.push("No results found across any search engine.".to_string());
        }

        Page {
            url: format!("search:multi:{}", self.query),
            title: format!("Search: {}", self.query),
            content_lines: lines,
            raw_content: String::new(),
            links,
        }
    }
}

impl Default for SearchManager {
    fn default() -> Self {
        Self::new().expect("Failed to create search manager")
    }
}

/// Helper function to convert HTML to plain text
fn html_to_text(html: &str) -> String {
    // Remove HTML tags
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(html, "").to_string()
}

/// Parse query to detect engine prefix
pub fn parse_query(input: &str) -> (Option<EngineType>, String) {
    let prefixes = ["exa:", "e:", "d:", "g:", "w:", "a:", "s:", "p:", "ol:"];

    for prefix in &prefixes {
        if input.to_lowercase().starts_with(prefix) {
            let query = input[prefix.len()..].trim().to_string();
            let engine = EngineType::from_prefix(prefix);
            return (engine, query);
        }
    }

    (None, input.to_string())
}

/// Classification of user input into a URL or a search query
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryTarget {
    Url(String),
    Search { engine: EngineType, query: String },
    MultiSearch { query: String },  // New: multi-engine search
}

/// Determine whether the input should be treated as a URL or a search query
pub fn classify_query(input: &str) -> QueryTarget {
    let trimmed = input.trim();

    // Respect explicit engine prefixes first
    if let (Some(engine), query) = parse_query(trimmed) {
        let engine = match engine {
            // Google results often require JS/consent; fall back to DuckDuckGo for reliability
            EngineType::Google => EngineType::DuckDuckGo,
            other => other,
        };

        return QueryTarget::Search { engine, query };
    }

    // Otherwise, detect URLs (with or without protocol)
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return QueryTarget::Url(trimmed.to_string());
    }

    if trimmed.contains('.') && !trimmed.contains(' ') {
        return QueryTarget::Url(format!("https://{}", trimmed));
    }

    // Default to multi-engine search (aggregated across all engines)
    QueryTarget::MultiSearch {
        query: trimmed.to_string(),
    }
}

/// Convert structured search results into a Page for display
pub fn results_to_page(response: SearchResponse) -> crate::browser::Page {
    use crate::browser::{Link, Page};

    let mut lines = vec![
        format!(
            "{} search results for \"{}\"",
            response.engine.name(),
            response.query
        ),
        String::new(),
    ];

    let mut links = Vec::new();

    for (idx, result) in response.results.iter().enumerate() {
        lines.push(format!("{}. {}", idx + 1, result.title));

        if !result.description.is_empty() {
            lines.push(format!("   {}", result.description));
        }

        lines.push(format!("   {}", result.url));
        lines.push(String::new());

        links.push(Link {
            text: format!("{} - {}", result.title, result.engine),
            url: result.url.clone(),
        });
    }

    if response.results.is_empty() {
        lines.push("No results found.".to_string());
    }

    Page {
        url: format!(
            "search:{}{}",
            response.engine.prefix(),
            response.query
        ),
        title: format!("{}: {}", response.engine.name(), response.query),
        content_lines: lines,
        raw_content: String::new(),
        links,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_prefixes() {
        assert_eq!(
            parse_query("w:rust"),
            (Some(EngineType::Wikipedia), "rust".to_string())
        );
        assert_eq!(
            parse_query("a:neural networks"),
            (Some(EngineType::ArXiv), "neural networks".to_string())
        );
        assert_eq!(
            parse_query("no prefix"),
            (None, "no prefix".to_string())
        );
    }

    #[test]
    fn test_engine_type_methods() {
        assert_eq!(EngineType::Wikipedia.name(), "Wikipedia");
        assert_eq!(EngineType::ArXiv.prefix(), "a:");
        assert_eq!(EngineType::from_prefix("w:"), Some(EngineType::Wikipedia));
    }

    #[test]
    fn test_html_to_text() {
        let html = "<span class=\"highlight\">Rust</span> is a programming language";
        assert_eq!(html_to_text(html), "Rust is a programming language");
    }

    #[test]
    fn test_classify_query() {
        match classify_query("example.com") {
            QueryTarget::Url(url) => assert!(url.starts_with("https://example.com")),
            other => panic!("expected URL, got {:?}", other),
        }

        match classify_query("w:rust") {
            QueryTarget::Search { engine, query } => {
                assert_eq!(engine, EngineType::Wikipedia);
                assert_eq!(query, "rust");
            }
            other => panic!("expected Wikipedia search, got {:?}", other),
        }

        match classify_query("g:rust") {
            QueryTarget::Search { engine, .. } => {
                assert_eq!(engine, EngineType::DuckDuckGo, "Google should fall back to DuckDuckGo");
            }
            other => panic!("expected search, got {:?}", other),
        }
    }
}
