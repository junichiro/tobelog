use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub dropbox_access_token: String,
    pub blog_title: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://blog.db".to_string()),
            dropbox_access_token: env::var("DROPBOX_ACCESS_TOKEN").unwrap_or_default(),
            blog_title: env::var("BLOG_TITLE").unwrap_or_else(|_| "My Personal Blog".to_string()),
        })
    }
}