use anyhow::{Context, Result};
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DropboxClient {
    client: Client,
    access_token: String,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub path_lower: String,
    pub path_display: String,
    pub size: Option<u64>,
    pub content_hash: Option<String>,
    pub client_modified: Option<String>,
    pub server_modified: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListFolderResult {
    pub entries: Vec<FileMetadata>,
    pub cursor: String,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
struct ListFolderRequest {
    path: String,
    recursive: bool,
    include_media_info: bool,
    include_deleted: bool,
}

#[derive(Debug, Serialize)]
struct DownloadRequest {
    path: String,
}

impl DropboxClient {
    pub fn new(access_token: String) -> Self {
        let client = Client::new();
        Self {
            client,
            access_token,
            base_url: "https://api.dropboxapi.com".to_string(),
        }
    }

    fn create_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        
        let auth_value = format!("Bearer {}", self.access_token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .context("Failed to create authorization header")?,
        );
        
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        Ok(headers)
    }

    fn create_auth_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", self.access_token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .context("Failed to create authorization header")?,
        );
        Ok(headers)
    }

    pub async fn test_connection(&self) -> Result<HashMap<String, serde_json::Value>> {
        let url = format!("{}/2/users/get_current_account", self.base_url);
        let headers = self.create_auth_headers()?;

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .send()
            .await
            .context("Failed to send test connection request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Dropbox API test connection failed with status {}: {}",
                status,
                error_text
            );
        }

        let account_info: HashMap<String, serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse account info response")?;

        Ok(account_info)
    }

    pub async fn list_folder(&self, path: &str) -> Result<ListFolderResult> {
        let url = format!("{}/2/files/list_folder", self.base_url);
        let headers = self.create_headers()?;

        let request_body = ListFolderRequest {
            path: path.to_string(),
            recursive: false,
            include_media_info: false,
            include_deleted: false,
        };

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send list folder request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Dropbox list folder failed with status {}: {}",
                status,
                error_text
            );
        }

        let result: ListFolderResult = response
            .json()
            .await
            .context("Failed to parse list folder response")?;

        Ok(result)
    }

    pub async fn download_file(&self, path: &str) -> Result<String> {
        let url = "https://content.dropboxapi.com/2/files/download";
        
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token))
                .context("Failed to create authorization header")?,
        );

        let dropbox_api_arg = serde_json::to_string(&DownloadRequest {
            path: path.to_string(),
        })?;
        
        headers.insert(
            "Dropbox-API-Arg",
            HeaderValue::from_str(&dropbox_api_arg)
                .context("Failed to create Dropbox-API-Arg header")?,
        );

        let response = self
            .client
            .post(url)
            .headers(headers)
            .send()
            .await
            .context("Failed to send download file request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Dropbox file download failed with status {}: {}",
                status,
                error_text
            );
        }

        let content = response
            .text()
            .await
            .context("Failed to read file content")?;

        Ok(content)
    }

    #[allow(dead_code)]
    pub async fn upload_file(&self, path: &str, content: &str) -> Result<FileMetadata> {
        let url = "https://content.dropboxapi.com/2/files/upload";
        
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.access_token))
                .context("Failed to create authorization header")?,
        );

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );

        let upload_args = serde_json::json!({
            "path": path,
            "mode": "overwrite",
            "autorename": false
        });

        headers.insert(
            "Dropbox-API-Arg",
            HeaderValue::from_str(&upload_args.to_string())
                .context("Failed to create Dropbox-API-Arg header")?,
        );

        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(content.to_string())
            .send()
            .await
            .context("Failed to send upload file request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Dropbox file upload failed with status {}: {}",
                status,
                error_text
            );
        }

        let metadata: FileMetadata = response
            .json()
            .await
            .context("Failed to parse upload response")?;

        Ok(metadata)
    }

    #[allow(dead_code)]
    pub async fn delete_file(&self, path: &str) -> Result<FileMetadata> {
        let url = format!("{}/2/files/delete_v2", self.base_url);
        let headers = self.create_headers()?;

        let request_body = serde_json::json!({
            "path": path
        });

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send delete file request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Dropbox file deletion failed with status {}: {}",
                status,
                error_text
            );
        }

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse delete response")?;

        let metadata: FileMetadata = serde_json::from_value(result["metadata"].clone())
            .context("Failed to extract metadata from delete response")?;

        Ok(metadata)
    }

    pub async fn create_folder(&self, path: &str) -> Result<FileMetadata> {
        let url = format!("{}/2/files/create_folder_v2", self.base_url);
        let headers = self.create_headers()?;

        let request_body = serde_json::json!({
            "path": path,
            "autorename": false
        });

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send create folder request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Dropbox folder creation failed with status {}: {}",
                status,
                error_text
            );
        }

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse create folder response")?;

        let metadata: FileMetadata = serde_json::from_value(result["metadata"].clone())
            .context("Failed to extract metadata from create folder response")?;

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dropbox_client_creation() {
        let client = DropboxClient::new("test_token".to_string());
        assert_eq!(client.access_token, "test_token");
        assert_eq!(client.base_url, "https://api.dropboxapi.com");
    }

    #[test]
    fn test_create_headers() {
        let client = DropboxClient::new("test_token".to_string());
        let headers = client.create_headers().unwrap();
        
        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
        
        let auth_header = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert_eq!(auth_header, "Bearer test_token");
    }
}