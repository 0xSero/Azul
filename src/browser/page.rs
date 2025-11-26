use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub url: String,
    pub title: String,
    pub content_lines: Vec<String>,
    pub raw_content: String,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub text: String,
    pub url: String,
}

impl Page {
    pub fn empty() -> Self {
        Self {
            url: String::new(),
            title: String::new(),
            content_lines: vec![],
            raw_content: String::new(),
            links: vec![],
        }
    }
}
