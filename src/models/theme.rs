use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Theme settings configuration for blog customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSettings {
    pub id: Option<i64>,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub primary_color: String,
    pub secondary_color: String,
    pub background_color: String,
    pub text_color: String,
    pub accent_color: String,
    pub font_family: String,
    pub heading_font: Option<String>,
    pub font_size_base: String,
    pub layout: ThemeLayout,
    pub dark_mode_enabled: bool,
    pub custom_css: Option<String>,
    pub header_style: HeaderStyle,
    pub footer_style: FooterStyle,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Layout configuration options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThemeLayout {
    #[serde(rename = "single")]
    Single, // Single column layout
    #[serde(rename = "sidebar")]
    Sidebar, // Two column with sidebar
    #[serde(rename = "magazine")]
    Magazine, // Magazine-style multi-column layout
}

/// Header style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderStyle {
    pub height: String,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub logo_position: String,    // "left", "center", "right"
    pub navigation_style: String, // "horizontal", "vertical", "hamburger"
    pub show_search: bool,
    pub sticky: bool,
}

/// Footer style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FooterStyle {
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub show_social_links: bool,
    pub show_copyright: bool,
    pub custom_content: Option<String>,
}

/// Theme creation/update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateThemeRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub primary_color: String,
    pub secondary_color: String,
    pub background_color: String,
    pub text_color: String,
    pub accent_color: String,
    pub font_family: String,
    pub heading_font: Option<String>,
    pub font_size_base: String,
    pub layout: ThemeLayout,
    pub dark_mode_enabled: bool,
    pub custom_css: Option<String>,
    pub header_style: HeaderStyle,
    pub footer_style: FooterStyle,
}

/// Update theme request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateThemeRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub accent_color: Option<String>,
    pub font_family: Option<String>,
    pub heading_font: Option<String>,
    pub font_size_base: Option<String>,
    pub layout: Option<ThemeLayout>,
    pub dark_mode_enabled: Option<bool>,
    pub custom_css: Option<String>,
    pub header_style: Option<HeaderStyle>,
    pub footer_style: Option<FooterStyle>,
}

/// Site configuration for global blog settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub id: Option<i64>,
    pub site_title: String,
    pub site_description: String,
    pub site_logo: Option<String>,
    pub favicon: Option<String>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub author_bio: Option<String>,
    pub social_links: Vec<SocialLink>,
    pub google_analytics_id: Option<String>,
    pub google_fonts: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Social media link configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialLink {
    pub platform: String, // "twitter", "github", "linkedin", etc.
    pub url: String,
    pub display_name: Option<String>,
    pub icon: Option<String>,
}

/// Dropbox template file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropboxTemplate {
    pub name: String,
    pub path: String,
    pub content: String,
    pub file_type: DropboxTemplateType,
    pub last_modified: DateTime<Utc>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DropboxTemplateType {
    #[serde(rename = "css")]
    Css,
    #[serde(rename = "theme")]
    Theme,
    #[serde(rename = "component")]
    Component,
    #[serde(rename = "font")]
    Font,
}

/// Theme filters for querying
#[derive(Debug, Clone, Default)]
pub struct ThemeFilters {
    pub is_active: Option<bool>,
    pub layout: Option<ThemeLayout>,
    pub dark_mode_enabled: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// CSS variable definition for theme customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssVariable {
    pub name: String,
    pub value: String,
    pub description: Option<String>,
    pub category: String, // "colors", "fonts", "spacing", "effects"
}

/// Response types for theme APIs
#[derive(Debug, Serialize)]
pub struct ThemeResponse {
    pub success: bool,
    pub data: ThemeSettings,
}

#[derive(Debug, Serialize)]
pub struct ThemeListResponse {
    pub success: bool,
    pub data: Vec<ThemeSettings>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SiteConfigResponse {
    pub success: bool,
    pub data: SiteConfig,
}

#[derive(Debug, Serialize)]
pub struct ThemePreviewResponse {
    pub success: bool,
    pub css: String,
    pub variables: Vec<CssVariable>,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            id: None,
            name: "default".to_string(),
            display_name: "Default Theme".to_string(),
            description: Some("Clean and professional default theme".to_string()),
            is_active: true,
            primary_color: "#3B82F6".to_string(),    // Blue-500
            secondary_color: "#6366F1".to_string(),  // Indigo-500
            background_color: "#FFFFFF".to_string(), // White
            text_color: "#1F2937".to_string(),       // Gray-800
            accent_color: "#F59E0B".to_string(),     // Amber-500
            font_family: "Inter, system-ui, sans-serif".to_string(),
            heading_font: Some("Inter, system-ui, sans-serif".to_string()),
            font_size_base: "16px".to_string(),
            layout: ThemeLayout::Sidebar,
            dark_mode_enabled: true,
            custom_css: None,
            header_style: HeaderStyle {
                height: "80px".to_string(),
                background_color: None,
                text_color: None,
                logo_position: "left".to_string(),
                navigation_style: "horizontal".to_string(),
                show_search: true,
                sticky: true,
            },
            footer_style: FooterStyle {
                background_color: None,
                text_color: None,
                show_social_links: true,
                show_copyright: true,
                custom_content: None,
            },
            created_at: None,
            updated_at: None,
        }
    }
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            id: None,
            site_title: "Tobelog".to_string(),
            site_description: "Personal Blog System built with Rust".to_string(),
            site_logo: None,
            favicon: None,
            author_name: "Blog Author".to_string(),
            author_email: None,
            author_bio: None,
            social_links: vec![],
            google_analytics_id: None,
            google_fonts: vec!["Inter:wght@400;500;600;700".to_string()],
            created_at: None,
            updated_at: None,
        }
    }
}

impl From<CreateThemeRequest> for ThemeSettings {
    fn from(req: CreateThemeRequest) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            name: req.name,
            display_name: req.display_name,
            description: req.description,
            is_active: false, // New themes are inactive by default
            primary_color: req.primary_color,
            secondary_color: req.secondary_color,
            background_color: req.background_color,
            text_color: req.text_color,
            accent_color: req.accent_color,
            font_family: req.font_family,
            heading_font: req.heading_font,
            font_size_base: req.font_size_base,
            layout: req.layout,
            dark_mode_enabled: req.dark_mode_enabled,
            custom_css: req.custom_css,
            header_style: req.header_style,
            footer_style: req.footer_style,
            created_at: Some(now),
            updated_at: Some(now),
        }
    }
}

impl ThemeSettings {
    /// Generate CSS variables from theme settings
    pub fn to_css_variables(&self) -> Vec<CssVariable> {
        vec![
            // Colors
            CssVariable {
                name: "--color-primary".to_string(),
                value: self.primary_color.clone(),
                description: Some("Primary brand color".to_string()),
                category: "colors".to_string(),
            },
            CssVariable {
                name: "--color-secondary".to_string(),
                value: self.secondary_color.clone(),
                description: Some("Secondary accent color".to_string()),
                category: "colors".to_string(),
            },
            CssVariable {
                name: "--color-background".to_string(),
                value: self.background_color.clone(),
                description: Some("Main background color".to_string()),
                category: "colors".to_string(),
            },
            CssVariable {
                name: "--color-text".to_string(),
                value: self.text_color.clone(),
                description: Some("Primary text color".to_string()),
                category: "colors".to_string(),
            },
            CssVariable {
                name: "--color-accent".to_string(),
                value: self.accent_color.clone(),
                description: Some("Accent color for highlights".to_string()),
                category: "colors".to_string(),
            },
            // Typography
            CssVariable {
                name: "--font-family-base".to_string(),
                value: format!("'{}'", self.font_family),
                description: Some("Base font family".to_string()),
                category: "fonts".to_string(),
            },
            CssVariable {
                name: "--font-family-heading".to_string(),
                value: format!(
                    "'{}'",
                    self.heading_font.as_ref().unwrap_or(&self.font_family)
                ),
                description: Some("Heading font family".to_string()),
                category: "fonts".to_string(),
            },
            CssVariable {
                name: "--font-size-base".to_string(),
                value: self.font_size_base.clone(),
                description: Some("Base font size".to_string()),
                category: "fonts".to_string(),
            },
            // Layout
            CssVariable {
                name: "--header-height".to_string(),
                value: self.header_style.height.clone(),
                description: Some("Header height".to_string()),
                category: "spacing".to_string(),
            },
        ]
    }

    /// Generate CSS content from theme settings
    pub fn to_css(&self) -> String {
        let variables = self.to_css_variables();
        let mut css = String::from(":root {\n");

        for var in variables {
            css.push_str(&format!("  {}: {};\n", var.name, var.value));
        }

        css.push_str("}\n\n");

        // Add layout-specific styles
        match self.layout {
            ThemeLayout::Single => {
                css.push_str(".layout-single { max-width: 800px; margin: 0 auto; }\n");
            }
            ThemeLayout::Sidebar => {
                css.push_str(".layout-sidebar { display: grid; grid-template-columns: 1fr 300px; gap: 2rem; }\n");
                css.push_str("@media (max-width: 768px) { .layout-sidebar { grid-template-columns: 1fr; } }\n");
            }
            ThemeLayout::Magazine => {
                css.push_str(".layout-magazine { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1.5rem; }\n");
            }
        }

        // Add custom CSS if provided
        if let Some(custom_css) = &self.custom_css {
            css.push('\n');
            css.push_str("/* Custom CSS */\n");
            css.push_str(custom_css);
        }

        css
    }
}
