use anyhow::Result;
use chrono::Utc;
use tracing::{debug, info, warn};

use crate::models::{
    ThemeSettings, CreateThemeRequest, UpdateThemeRequest, SiteConfig, 
    DropboxTemplate, DropboxTemplateType, ThemeFilters, CssVariable
};
use crate::services::{DatabaseService, DropboxClient};

/// Service for managing blog themes and custom design features
#[derive(Clone)]
pub struct ThemeService {
    database: DatabaseService,
    dropbox: std::sync::Arc<DropboxClient>,
}

impl ThemeService {
    /// Create a new theme service
    pub fn new(database: DatabaseService, dropbox: std::sync::Arc<DropboxClient>) -> Self {
        Self {
            database,
            dropbox,
        }
    }

    /// Get the currently active theme
    pub async fn get_active_theme(&self) -> Result<ThemeSettings> {
        debug!("Getting active theme");
        
        match self.database.get_active_theme().await? {
            Some(theme) => {
                debug!("Found active theme: {}", theme.name);
                Ok(theme)
            }
            None => {
                info!("No active theme found, creating default theme");
                let default_theme = ThemeSettings::default();
                let created_theme = self.database.create_theme(&default_theme).await?;
                self.database.activate_theme(&created_theme.name).await?;
                Ok(created_theme)
            }
        }
    }

    /// Create a new theme
    pub async fn create_theme(&self, request: CreateThemeRequest) -> Result<ThemeSettings> {
        debug!("Creating new theme: {}", request.name);

        // Validate theme name uniqueness
        if self.database.get_theme_by_name(&request.name).await?.is_some() {
            return Err(anyhow::anyhow!("Theme with name '{}' already exists", request.name));
        }

        let theme = ThemeSettings::from(request);
        let created_theme = self.database.create_theme(&theme).await?;
        
        info!("Created theme: {} ({})", created_theme.display_name, created_theme.name);
        Ok(created_theme)
    }

    /// Update an existing theme
    pub async fn update_theme(&self, name: &str, request: UpdateThemeRequest) -> Result<ThemeSettings> {
        debug!("Updating theme: {}", name);

        let updated_theme = self.database.update_theme(name, request).await?
            .ok_or_else(|| anyhow::anyhow!("Theme '{}' not found", name))?;

        info!("Updated theme: {}", name);
        Ok(updated_theme)
    }

    /// Delete a theme
    pub async fn delete_theme(&self, name: &str) -> Result<bool> {
        debug!("Deleting theme: {}", name);

        // Prevent deletion of active theme
        let active_theme = self.get_active_theme().await?;
        if active_theme.name == name {
            return Err(anyhow::anyhow!("Cannot delete active theme. Please activate another theme first."));
        }

        let deleted = self.database.delete_theme(name).await?;
        if deleted {
            info!("Deleted theme: {}", name);
        }
        Ok(deleted)
    }

    /// Set a theme as active
    pub async fn set_active_theme(&self, name: &str) -> Result<ThemeSettings> {
        debug!("Setting active theme: {}", name);

        // Deactivate current active theme
        if let Ok(current_active) = self.get_active_theme().await {
            self.database.deactivate_theme(&current_active.name).await?;
        }

        // Activate the new theme
        let theme = self.database.activate_theme(name).await?
            .ok_or_else(|| anyhow::anyhow!("Theme '{}' not found", name))?;

        info!("Activated theme: {}", name);
        Ok(theme)
    }

    /// List all themes with optional filters
    pub async fn list_themes(&self, filters: ThemeFilters) -> Result<Vec<ThemeSettings>> {
        debug!("Listing themes with filters: {:?}", filters);
        self.database.list_themes(filters).await
    }

    /// Get theme by name
    pub async fn get_theme(&self, name: &str) -> Result<Option<ThemeSettings>> {
        debug!("Getting theme: {}", name);
        self.database.get_theme_by_name(name).await
    }

    /// Generate CSS for a theme
    pub async fn generate_theme_css(&self, name: &str) -> Result<String> {
        debug!("Generating CSS for theme: {}", name);

        let theme = self.get_theme(name).await?
            .ok_or_else(|| anyhow::anyhow!("Theme '{}' not found", name))?;

        // Start with base theme CSS
        let mut css = theme.to_css();

        // Load custom CSS from Dropbox if available
        if let Ok(dropbox_css) = self.load_dropbox_css(&theme.name).await {
            css.push('\n');
            css.push_str("/* Dropbox Custom CSS */\n");
            css.push_str(&dropbox_css);
        }

        // Load theme-specific CSS from Dropbox
        if let Ok(theme_css) = self.load_theme_css(&theme.name).await {
            css.push('\n');
            css.push_str("/* Theme-specific CSS */\n");
            css.push_str(&theme_css);
        }

        Ok(css)
    }

    /// Load CSS from Dropbox template folder
    async fn load_dropbox_css(&self, theme_name: &str) -> Result<String> {
        debug!("Loading CSS from Dropbox for theme: {}", theme_name);

        let css_path = "/BlogStorage/templates/style.css".to_string();
        
        match self.dropbox.download_file(&css_path).await {
            Ok(content) => {
                debug!("Loaded main CSS from Dropbox: {} bytes", content.len());
                Ok(String::from_utf8(content)?)
            }
            Err(e) => {
                debug!("No main CSS found in Dropbox: {}", e);
                Ok(String::new())
            }
        }
    }

    /// Load theme-specific CSS from Dropbox
    async fn load_theme_css(&self, theme_name: &str) -> Result<String> {
        debug!("Loading theme-specific CSS from Dropbox: {}", theme_name);

        let theme_css_path = format!("/BlogStorage/templates/themes/{}.css", theme_name);
        
        match self.dropbox.download_file(&theme_css_path).await {
            Ok(content) => {
                debug!("Loaded theme CSS from Dropbox: {} bytes", content.len());
                Ok(String::from_utf8(content)?)
            }
            Err(e) => {
                debug!("No theme-specific CSS found in Dropbox: {}", e);
                Ok(String::new())
            }
        }
    }

    /// Sync themes from Dropbox templates folder
    pub async fn sync_dropbox_themes(&self) -> Result<Vec<DropboxTemplate>> {
        debug!("Syncing themes from Dropbox");

        let templates_path = "/BlogStorage/templates";
        let mut synced_templates = Vec::new();

        // List files in templates directory
        match self.dropbox.list_folder(templates_path).await {
            Ok(response) => {
                for entry in &response.entries {
                    // Process CSS files
                    if entry.name.ends_with(".css") {
                        match self.process_dropbox_css_file(&entry.name, &entry.path_lower).await {
                            Ok(template) => synced_templates.push(template),
                            Err(e) => warn!("Failed to process CSS file {}: {}", entry.name, e),
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to list Dropbox templates folder: {}", e);
            }
        }

        // Check themes subfolder
        match self.sync_themes_subfolder().await {
            Ok(mut theme_templates) => synced_templates.append(&mut theme_templates),
            Err(e) => warn!("Failed to sync themes subfolder: {}", e),
        }

        info!("Synced {} templates from Dropbox", synced_templates.len());
        Ok(synced_templates)
    }

    /// Sync themes from the themes subfolder
    async fn sync_themes_subfolder(&self) -> Result<Vec<DropboxTemplate>> {
        debug!("Syncing themes from Dropbox themes subfolder");

        let themes_path = "/BlogStorage/templates/themes";
        let mut theme_templates = Vec::new();

        match self.dropbox.list_folder(themes_path).await {
            Ok(response) => {
                for entry in &response.entries {
                    if entry.name.ends_with(".css") {
                        match self.process_dropbox_css_file(&entry.name, &entry.path_lower).await {
                            Ok(template) => theme_templates.push(template),
                            Err(e) => warn!("Failed to process theme CSS file {}: {}", entry.name, e),
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Themes subfolder not found or empty: {}", e);
            }
        }

        Ok(theme_templates)
    }

    /// Process a CSS file from Dropbox
    async fn process_dropbox_css_file(&self, name: &str, path: &str) -> Result<DropboxTemplate> {
        debug!("Processing Dropbox CSS file: {}", name);

        let content = self.dropbox.download_file(path).await?;
        let content_str = String::from_utf8(content.clone())?;

        // Determine template type based on location and name
        let file_type = if path.contains("/themes/") {
            DropboxTemplateType::Theme
        } else if name.contains("component") {
            DropboxTemplateType::Component
        } else if name.contains("font") {
            DropboxTemplateType::Font
        } else {
            DropboxTemplateType::Css
        };

        Ok(DropboxTemplate {
            name: name.to_string(),
            path: path.to_string(),
            content: content_str,
            file_type,
            last_modified: Utc::now(), // TODO: Get actual modification time from Dropbox
            size: content.len() as u64,
        })
    }

    /// Get site configuration
    pub async fn get_site_config(&self) -> Result<SiteConfig> {
        debug!("Getting site configuration");

        match self.database.get_site_config().await? {
            Some(config) => Ok(config),
            None => {
                info!("No site config found, creating default");
                let default_config = SiteConfig::default();
                self.database.create_site_config(&default_config).await
            }
        }
    }

    /// Update site configuration
    pub async fn update_site_config(&self, config: SiteConfig) -> Result<SiteConfig> {
        debug!("Updating site configuration");
        self.database.update_site_config(config).await
    }

    /// Validate CSS content
    pub fn validate_css(&self, css: &str) -> Result<Vec<String>> {
        debug!("Validating CSS content");

        let mut warnings = Vec::new();

        // Basic CSS validation - check for common issues
        if css.contains("@import") {
            warnings.push("@import statements may cause performance issues".to_string());
        }

        if css.contains("javascript:") {
            return Err(anyhow::anyhow!("CSS contains potentially unsafe javascript: URLs"));
        }

        // Check for unclosed braces
        let open_braces = css.matches('{').count();
        let close_braces = css.matches('}').count();
        if open_braces != close_braces {
            warnings.push(format!("Mismatched braces: {} open, {} close", open_braces, close_braces));
        }

        Ok(warnings)
    }

    /// Get theme preview with compiled CSS
    pub async fn get_theme_preview(&self, name: &str) -> Result<(String, Vec<CssVariable>)> {
        debug!("Getting theme preview: {}", name);

        let theme = self.get_theme(name).await?
            .ok_or_else(|| anyhow::anyhow!("Theme '{}' not found", name))?;

        let css = self.generate_theme_css(name).await?;
        let variables = theme.to_css_variables();

        Ok((css, variables))
    }

    /// Create preset themes (default, dark, minimal)
    pub async fn create_preset_themes(&self) -> Result<()> {
        info!("Creating preset themes");

        // Dark theme
        let dark_theme = CreateThemeRequest {
            name: "dark".to_string(),
            display_name: "Dark Theme".to_string(),
            description: Some("Elegant dark theme for night reading".to_string()),
            primary_color: "#60A5FA".to_string(),     // Blue-400
            secondary_color: "#A78BFA".to_string(),   // Violet-400
            background_color: "#111827".to_string(),  // Gray-900
            text_color: "#F9FAFB".to_string(),       // Gray-50
            accent_color: "#FBBF24".to_string(),     // Yellow-400
            font_family: "Inter, system-ui, sans-serif".to_string(),
            heading_font: Some("Inter, system-ui, sans-serif".to_string()),
            font_size_base: "16px".to_string(),
            layout: crate::models::ThemeLayout::Sidebar,
            dark_mode_enabled: true,
            custom_css: Some("body { background: linear-gradient(135deg, #111827 0%, #1F2937 100%); }".to_string()),
            header_style: crate::models::HeaderStyle {
                height: "80px".to_string(),
                background_color: Some("#1F2937".to_string()),
                text_color: Some("#F9FAFB".to_string()),
                logo_position: "left".to_string(),
                navigation_style: "horizontal".to_string(),
                show_search: true,
                sticky: true,
            },
            footer_style: crate::models::FooterStyle {
                background_color: Some("#1F2937".to_string()),
                text_color: Some("#F9FAFB".to_string()),
                show_social_links: true,
                show_copyright: true,
                custom_content: None,
            },
        };

        // Minimal theme
        let minimal_theme = CreateThemeRequest {
            name: "minimal".to_string(),
            display_name: "Minimal Theme".to_string(),
            description: Some("Clean and minimalist design focused on content".to_string()),
            primary_color: "#374151".to_string(),     // Gray-700
            secondary_color: "#6B7280".to_string(),   // Gray-500
            background_color: "#FFFFFF".to_string(),  // White
            text_color: "#111827".to_string(),        // Gray-900
            accent_color: "#DC2626".to_string(),      // Red-600
            font_family: "Georgia, serif".to_string(),
            heading_font: Some("Inter, system-ui, sans-serif".to_string()),
            font_size_base: "18px".to_string(),
            layout: crate::models::ThemeLayout::Single,
            dark_mode_enabled: false,
            custom_css: Some(".article { line-height: 1.8; } h1, h2, h3 { font-weight: 300; }".to_string()),
            header_style: crate::models::HeaderStyle {
                height: "60px".to_string(),
                background_color: None,
                text_color: None,
                logo_position: "center".to_string(),
                navigation_style: "horizontal".to_string(),
                show_search: false,
                sticky: false,
            },
            footer_style: crate::models::FooterStyle {
                background_color: None,
                text_color: None,
                show_social_links: false,
                show_copyright: true,
                custom_content: None,
            },
        };

        // Create themes if they don't exist
        for theme_req in [dark_theme, minimal_theme] {
            if self.get_theme(&theme_req.name).await?.is_none() {
                match self.create_theme(theme_req.clone()).await {
                    Ok(_) => info!("Created preset theme: {}", theme_req.name),
                    Err(e) => warn!("Failed to create preset theme {}: {}", theme_req.name, e),
                }
            }
        }

        Ok(())
    }
}

impl From<ThemeSettings> for CreateThemeRequest {
    fn from(theme: ThemeSettings) -> Self {
        Self {
            name: theme.name,
            display_name: theme.display_name,
            description: theme.description,
            primary_color: theme.primary_color,
            secondary_color: theme.secondary_color,
            background_color: theme.background_color,
            text_color: theme.text_color,
            accent_color: theme.accent_color,
            font_family: theme.font_family,
            heading_font: theme.heading_font,
            font_size_base: theme.font_size_base,
            layout: theme.layout,
            dark_mode_enabled: theme.dark_mode_enabled,
            custom_css: theme.custom_css,
            header_style: theme.header_style,
            footer_style: theme.footer_style,
        }
    }
}