#![cfg(feature = "network-tests")]

use std::process::Command;
use std::time::Duration;

#[test]
#[ignore = "network-bound; run with --features network-tests -- --ignored"]
fn test_cli_search() {
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "-q", "rust programming"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Should have output");
    assert!(
        stdout.contains("Azul CLI Mode"),
        "Should show CLI mode header"
    );
    assert!(
        stdout.contains("Searching DuckDuckGo for"),
        "Should perform a DuckDuckGo search"
    );
}

#[test]
#[ignore = "network-bound; run with --features network-tests -- --ignored"]
fn test_cli_url_fetch() {
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "-q", "https://example.com"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Example Domain"),
        "Should fetch example.com"
    );
}

#[test]
#[ignore = "network-bound; run with --features network-tests -- --ignored"]
fn test_cli_wikipedia_search() {
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "-q", "w:Rust"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Searching Wikipedia for"),
        "Should search Wikipedia"
    );
}

#[test]
#[ignore = "network-bound; run with --features network-tests -- --ignored"]
fn test_cli_arxiv_search() {
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "-q", "a:neural networks"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Searching arXiv for"),
        "Should search arXiv"
    );
}

#[test]
#[ignore = "network-bound; run with --features network-tests -- --ignored"]
fn test_http_client() {
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "--test-http"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "HTTP test should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("All HTTP tests passed!"),
        "All HTTP tests should pass"
    );
}

#[test]
#[ignore = "network-bound; run with --features network-tests -- --ignored"]
fn test_browser_client() {
    // Test that we can create a browser client
    use reqwest::blocking::Client;

    let client = Client::builder().timeout(Duration::from_secs(10)).build();

    assert!(client.is_ok(), "Should create HTTP client");
}

#[test]
#[ignore = "network-bound; run with --features network-tests -- --ignored"]
fn test_simple_fetch() {
    use reqwest::blocking::Client;

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build client");

    let response = client.get("https://example.com").send();
    assert!(response.is_ok(), "Should fetch example.com");
}

#[test]
#[ignore = "network-bound; run with --features network-tests -- --ignored"]
fn test_search_prefix_parsing() {
    // Test that search prefixes are correctly parsed
    // This tests the search routing logic

    let test_cases = vec![
        ("w:test", "Searching Wikipedia for"),
        ("a:test", "Searching arXiv for"),
        ("s:test", "Searching Google Scholar for"),
        ("d:test", "Searching DuckDuckGo for"),
        ("g:test", "Searching DuckDuckGo for"),
    ];

    for (input, expected_domain) in test_cases {
        let output = Command::new("cargo")
            .args(["run", "--release", "--", "-q", input])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected_domain),
            "Input '{}' should use {}",
            input,
            expected_domain
        );
    }
}
