mod ai;
mod app;
mod browser;
mod chat;
mod config;
mod mascot;
mod memory;
mod rag;
mod scrape;
mod search;
mod storage;
mod tabs;
mod ui;

use anyhow::{Context, Result};
use app::App;
use browser::{Browser, RenderMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::io;
use std::time::{Duration, Instant};
use search::{QueryTarget, SearchManager};

const VERSION: &str = "3.0.0";

fn print_help() {
    println!(
        r#"Azul-Browse v{} - Terminal Web Browser

USAGE:
    azul-browse [OPTIONS]
    azul-browse -q <QUERY>
    azul-browse --js -q <URL>

OPTIONS:
    -q, --query <QUERY>    Search or fetch URL in CLI mode
    --js                   Enable JavaScript rendering (headless Chrome)
    --test-http            Run HTTP client tests
    -h, --help             Show this help message
    -v, --version          Show version

SEARCH PREFIXES:
    w:<query>    Wikipedia search
    a:<query>    arXiv search (academic papers)
    s:<query>    Google Scholar search
    d:<query>    DuckDuckGo search
    g:<query>    General web search (DuckDuckGo)
    p:<query>    PubMed search (medical)
    ol:<query>   OpenLibrary search (books)

    Without prefix, searches DuckDuckGo by default.
    Direct URLs (containing '.' without spaces) go directly to the site.

TUI KEYBINDINGS:
    /           Focus URL bar
    Tab         Cycle focus (Content -> Sidebar -> URL Bar)
    j/k, ↓/↑    Scroll content or navigate links
    g/G         Go to top/bottom of content
    Enter       Open selected link (in sidebar)
    1, 2        Focus Content (1) or Sidebar (2)
    ?           Toggle help
    q, Ctrl+C   Quit

EXAMPLES:
    azul-browse                           # Start TUI browser
    azul-browse -q "rust programming"     # Search DuckDuckGo
    azul-browse -q "w:Rust language"      # Search Wikipedia
    azul-browse -q "a:neural networks"    # Search arXiv
    azul-browse -q "https://example.com"  # Fetch URL directly
    azul-browse --test-http               # Test HTTP client

ENVIRONMENT VARIABLES:
    OPENROUTER_API_KEY    API key for AI summaries (optional)
"#,
        VERSION
    );
}

fn print_version() {
    println!("azul-browse {}", VERSION);
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Check for --js flag
    let js_mode = args.iter().any(|a| a == "--js");

    // Parse arguments
    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "-v" | "--version" => {
                print_version();
                return Ok(());
            }
            "-q" | "--query" => {
                if let Some(query) = args.get(i + 1) {
                    if query == "--js" {
                        // --js came after -q, get next arg
                        if let Some(q) = args.get(i + 2) {
                            return run_cli_mode(q, js_mode);
                        }
                    }
                    return run_cli_mode(query, js_mode);
                } else {
                    eprintln!("Error: -q requires a query argument");
                    return Ok(());
                }
            }
            "--test-http" => {
                return test_http_client();
            }
            _ => {}
        }
    }

    // Legacy support: -q query (where query is multiple args)
    if args.len() >= 3 && args[1] == "-q" {
        let query_parts: Vec<&String> = args[2..].iter().filter(|a| *a != "--js").collect();
        let query = query_parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
        return run_cli_mode(&query, js_mode);
    }

    // Start TUI mode
    run_tui()
}

fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;
    let tick_rate = Duration::from_millis(app.config.refresh_rate_ms);

    // Run app
    let res = run_app(&mut terminal, &mut app, tick_rate);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
) -> Result<()> {
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Handle quit specially
                if matches!(key.code, KeyCode::Char('c'))
                    && key.modifiers.contains(event::KeyModifiers::CONTROL)
                {
                    app.quit();
                } else {
                    app.handle_key(key)?;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update();
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// CLI mode - fetch a URL or search query and display results
fn run_cli_mode(query: &str, js_mode: bool) -> Result<()> {
    println!("Azul CLI Mode v{}", VERSION);
    println!("Query: {}", query);
    if js_mode {
        println!("Mode: JavaScript rendering (headless Chrome)");
    }
    println!("---");

    let render_mode = if js_mode {
        RenderMode::JavaScript
    } else {
        RenderMode::Static
    };

    match search::classify_query(query) {
        QueryTarget::Url(url) => {
            println!("Fetching{}: {}", if js_mode { " [JS]" } else { "" }, url);
            println!();

            let browser = Browser::new().context("Failed to create browser")?;
            let page = browser.fetch_with_mode(&url, render_mode).context("Failed to fetch page")?;

            print_page(&page);
        }
        QueryTarget::Search { engine, query } => {
            println!("Searching {} for: {}", engine.name(), query);
            println!();

            let response = SearchManager::new()
                .and_then(|manager| manager.search_with(engine, &query))
                .context("Search failed")?;

            let page = search::results_to_page(response);
            print_page(&page);
        }
    }

    Ok(())
}

fn print_page(page: &browser::Page) {
    println!("Title: {}", page.title);
    println!("URL: {}", page.url);
    println!("Links found: {}", page.links.len());
    println!();
    println!("--- Content Preview (first 50 lines) ---");
    for (i, line) in page.content_lines.iter().take(50).enumerate() {
        println!("{:4}: {}", i + 1, line);
    }

    if page.content_lines.len() > 50 {
        println!("... ({} more lines)", page.content_lines.len() - 50);
    }

    println!();
    println!("--- Links ---");
    for (i, link) in page.links.iter().take(20).enumerate() {
        println!("{:3}. {} -> {}", i + 1, link.text, link.url);
    }

    if page.links.len() > 20 {
        println!("... ({} more links)", page.links.len() - 20);
    }
}

/// Test HTTP client creation and basic fetch
fn test_http_client() -> Result<()> {
    println!("Testing HTTP client...");
    println!();

    // Test 1: Create client with minimal options
    println!("Test 1: Creating minimal reqwest client...");
    let _client = reqwest::blocking::Client::builder()
        .build()
        .context("Failed to create minimal client")?;
    println!("  OK: Minimal client created");

    // Test 2: Create client with timeout
    println!("Test 2: Creating client with timeout...");
    let _client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create client with timeout")?;
    println!("  OK: Client with timeout created");

    // Test 3: Create client with user agent
    println!("Test 3: Creating client with user agent...");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Azul-Browse/3.0 (Terminal Browser)")
        .build()
        .context("Failed to create client with user agent")?;
    println!("  OK: Client with user agent created");

    // Test 4: Fetch example.com
    println!("Test 4: Fetching example.com...");
    let response = client
        .get("https://example.com")
        .send()
        .context("Failed to send request")?;
    println!("  OK: Got response with status {}", response.status());

    // Test 5: Read response body
    println!("Test 5: Reading response body...");
    let body = response.text().context("Failed to read body")?;
    println!("  OK: Body length = {} bytes", body.len());

    // Test 6: Test Browser struct
    println!("Test 6: Creating Browser struct...");
    let browser = Browser::new().context("Failed to create Browser")?;
    println!("  OK: Browser created");

    // Test 7: Fetch via Browser
    println!("Test 7: Fetching example.com via Browser...");
    let page = browser
        .fetch("https://example.com")
        .context("Failed to fetch via Browser")?;
    println!("  OK: Page fetched - title: {}", page.title);
    println!("  Content lines: {}", page.content_lines.len());
    println!("  Links found: {}", page.links.len());

    println!();
    println!("All HTTP tests passed!");

    Ok(())
}
