use reqwest::blocking::Client;
use std::time::Duration;

fn main() {
    println!("Testing browser creation...");
    
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Azul-Browse/3.0")
        .build();
    
    match client {
        Ok(c) => {
            println!("✓ Client created successfully");
            println!("Testing fetch...");
            match c.get("https://example.com").send() {
                Ok(resp) => println!("✓ Fetch successful: status {}", resp.status()),
                Err(e) => println!("✗ Fetch failed: {}", e),
            }
        }
        Err(e) => println!("✗ Client creation failed: {}", e),
    }
}
