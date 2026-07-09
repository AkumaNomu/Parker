#[cfg(feature = "site_retriever")]
use kuchiki::{traits::*, NodeRef, ElementData, Attributes, Node};
#[cfg(feature = "site_retriever")]
use regex::Regex;
#[cfg(feature = "site_retriever")]
use reqwest::blocking::Client;
#[cfg(feature = "site_retriever")]
use std::collections::HashMap;
#[cfg(feature = "site_retriever")]
use std::fs;
#[cfg(feature = "site_retriever")]
use std::path::{Path, PathBuf};

#[cfg(feature = "site_retriever")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtractedComponent {
    pub selector: String,
    pub html: String,
    pub outer_html: String,
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
    pub computed_styles: HashMap<String, String>,
    pub bounding_box: Option<BoundingBox>,
    pub children: Vec<ExtractedComponent>,
}

#[cfg(feature = "site_retriever")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[cfg(feature = "site_retriever")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct PageAssets {
    pub css: Vec<Asset>,
    pub js: Vec<Asset>,
    pub images: Vec<Asset>,
    pub fonts: Vec<Asset>,
    pub other: Vec<Asset>,
}

#[cfg(feature = "site_retriever")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Asset {
    pub url: String,
    pub local_path: Option<String>,
    pub content_type: Option<String>,
    pub size: Option<u64>,
}

#[cfg(feature = "site_retriever")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtractedPage {
    pub url: String,
    pub title: String,
    pub html: String,
    pub components: Vec<ExtractedComponent>,
    pub assets: PageAssets,
    pub viewport: Viewport,
}

#[cfg(feature = "site_retriever")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[cfg(feature = "site_retriever")]
pub struct SiteRetriever {
    client: Client,
    output_dir: PathBuf,
    download_assets: bool,
    max_depth: usize,
}

#[cfg(feature = "site_retriever")]
impl SiteRetriever {
    pub fn new(output_dir: PathBuf, download_assets: bool) -> Result<Self, String> {
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Parker/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            output_dir,
            download_assets,
            max_depth: 3,
        })
    }

    pub fn extract(&self, url: &str) -> Result<ExtractedPage, String> {
        let html = self.fetch_html(url)?;
        let document = kuchiki::parse_html().one(html.clone());
        
        let title = self.extract_title(&document);
        let components = self.extract_components(&document)?;
        let assets = self.extract_assets(&document, url)?;
        let viewport = Viewport { width: 1920, height: 1080 };

        Ok(ExtractedPage {
            url: url.to_string(),
            title,
            html,
            components,
            assets,
            viewport,
        })
    }

    fn fetch_html(&self, url: &str) -> Result<String, String> {
        let response = self.client
            .get(url)
            .send()
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response.text()
            .map_err(|e| format!("Failed to read response body: {}", e))
    }

    fn extract_title(&self, document: &NodeRef) -> String {
        document
            .select("title")
            .ok()
            .and_then(|mut iter| iter.next())
            .map(|node| node.text_contents())
            .unwrap_or_default()
    }

    fn extract_components(&self, document: &NodeRef) -> Result<Vec<ExtractedComponent>, String> {
        let mut components = Vec::new();
        
        let selector = document.select("body *");
        if selector.is_err() {
            return Err("Selector error".into());
        }
        
        for node in selector.unwrap() {
            if let Some(component) = self.node_to_component(&node.as_node(), 0)? {
                components.push(component);
            }
        }

        Ok(components)
    }

    fn node_to_component(&self, node: &NodeRef, depth: usize) -> Result<Option<ExtractedComponent>, String> {
        if depth > self.max_depth {
            return Ok(None);
        }

        let element = match node.as_element() {
            Some(el) => el,
            None => return Ok(None),
        };

        let tag_name = element.name.local.to_string();
        
        if matches!(tag_name.as_str(), "script" | "style" | "noscript" | "head" | "meta" | "link") {
            return Ok(None);
        }

        let mut attributes = HashMap::new();
        let attrs = element.attributes.borrow();
        if let Some(id) = attrs.get("id") {
            attributes.insert("id".to_string(), id.to_string());
        }
        if let Some(class) = attrs.get("class") {
            attributes.insert("class".to_string(), class.to_string());
        }
        if let Some(href) = attrs.get("href") {
            attributes.insert("href".to_string(), href.to_string());
        }
        if let Some(src) = attrs.get("src") {
            attributes.insert("src".to_string(), src.to_string());
        }
        if let Some(rel) = attrs.get("rel") {
            attributes.insert("rel".to_string(), rel.to_string());
        }
        if let Some(as_attr) = attrs.get("as") {
            attributes.insert("as".to_string(), as_attr.to_string());
        }

        let outer_html = node.to_string();
        let inner_html = node
            .children()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("");

        let mut children = Vec::new();
        for child in node.children() {
            if let Some(comp) = self.node_to_component(&child, depth + 1)? {
                children.push(comp);
            }
        }

        let selector = self.generate_selector(node, element);

        Ok(Some(ExtractedComponent {
            selector,
            html: inner_html,
            outer_html,
            tag_name,
            attributes,
            computed_styles: HashMap::new(),
            bounding_box: None,
            children,
        }))
    }

    fn generate_selector(&self, node: &NodeRef, element: &ElementData) -> String {
        let attrs = element.attributes.borrow();
        let mut parts = Vec::new();
        
        if let Some(id) = attrs.get("id") {
            if !id.contains(' ') && !id.is_empty() {
                parts.push(format!("#{}", id));
                return parts.join(" ");
            }
        }

        if let Some(class) = attrs.get("class") {
            let classes: Vec<&str> = class.split_whitespace().collect();
            if !classes.is_empty() {
                parts.push(format!(".{}", classes.join(".")));
                return parts.join(" ");
            }
        }

        parts.push(element.name.local.to_string());
        
        if let Some(parent_node) = node.parent() {
            if let Some(parent_elem) = parent_node.as_element() {
                let parent_attrs = parent_elem.attributes.borrow();
                let parent_sel = self.generate_selector_from_attrs(&parent_attrs);
                if !parent_sel.is_empty() {
                    parts.insert(0, parent_sel);
                }
            }
        }

        parts.join(" > ")
    }

    fn generate_selector_from_attrs(&self, attrs: &Attributes) -> String {
        let mut parts = Vec::new();
        
        if let Some(id) = attrs.get("id") {
            if !id.contains(' ') && !id.is_empty() {
                parts.push(format!("#{}", id));
                return parts.join(" ");
            }
        }

        if let Some(class) = attrs.get("class") {
            let classes: Vec<&str> = class.split_whitespace().collect();
            if !classes.is_empty() {
                parts.push(format!(".{}", classes.join(".")));
                return parts.join(" ");
            }
        }

        parts.join(" > ")
    }

    fn extract_assets(&self, document: &NodeRef, base_url: &str) -> Result<PageAssets, String> {
        let base = reqwest::Url::parse(base_url)
            .map_err(|e| format!("Invalid base URL: {}", e))?;

        let mut css = Vec::new();
        let mut js = Vec::new();
        let mut images = Vec::new();
        let mut fonts = Vec::new();
        let mut other = Vec::new();

        if let Ok(selection) = document.select("link[href]") {
            for link in selection {
                let attrs = link.attributes.borrow();
                if let Some(href) = attrs.get("href") {
                    let rel = attrs.get("rel").unwrap_or("");
                    let abs_url = base.join(href).map(|u| u.to_string()).unwrap_or(href.to_string());
                    
                    if rel.contains("stylesheet") {
                        css.push(Asset { url: abs_url, local_path: None, content_type: Some("text/css".into()), size: None });
                    } else if rel.contains("preload") {
                        if attrs.get("as") == Some("font") {
                            fonts.push(Asset { url: abs_url, local_path: None, content_type: Some("font".into()), size: None });
                        }
                    } else {
                        other.push(Asset { url: abs_url, local_path: None, content_type: None, size: None });
                    }
                }
            }
        }

        if let Ok(selection) = document.select("script[src]") {
            for script in selection {
                let attrs = script.attributes.borrow();
                if let Some(src) = attrs.get("src") {
                    let abs_url = base.join(src).map(|u| u.to_string()).unwrap_or(src.to_string());
                    js.push(Asset { url: abs_url, local_path: None, content_type: Some("application/javascript".into()), size: None });
                }
            }
        }

        if let Ok(selection) = document.select("img[src]") {
            for img in selection {
                let attrs = img.attributes.borrow();
                if let Some(src) = attrs.get("src") {
                    let abs_url = base.join(src).map(|u| u.to_string()).unwrap_or(src.to_string());
                    images.push(Asset { url: abs_url, local_path: None, content_type: Some("image".into()), size: None });
                }
            }
        }

        if let Ok(selection) = document.select("source[src]") {
            for source in selection {
                let attrs = source.attributes.borrow();
                if let Some(src) = attrs.get("src") {
                    let abs_url = base.join(src).map(|u| u.to_string()).unwrap_or(src.to_string());
                    other.push(Asset { url: abs_url, local_path: None, content_type: Some("media".into()), size: None });
                }
            }
        }

        Ok(PageAssets { css, js, images, fonts, other })
    }

    pub fn download_assets(&self, assets: &mut PageAssets) -> Result<(), String> {
        if !self.download_assets {
            return Ok(());
        }

        let asset_dirs = [
            (&mut assets.css, "css"),
            (&mut assets.js, "js"),
            (&mut assets.images, "images"),
            (&mut assets.fonts, "fonts"),
            (&mut assets.other, "other"),
        ];

        for (asset_list, subdir) in asset_dirs {
            let dir = self.output_dir.join(subdir);
            fs::create_dir_all(&dir).map_err(|e| format!("Failed to create asset dir: {}", e))?;

            for asset in asset_list.iter_mut() {
                if let Ok(local_path) = self.download_asset(&asset.url, &dir) {
                    asset.local_path = Some(local_path);
                }
            }
        }

        Ok(())
    }

    fn download_asset(&self, url: &str, dir: &Path) -> Result<String, String> {
        let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
        let filename = parsed.path_segments()
            .and_then(|mut seg| seg.next_back())
            .filter(|s| !s.is_empty())
            .unwrap_or("asset");
        
        let filename = sanitize_filename(filename);
        let local_path = dir.join(&filename);

        let response = self.client.get(url).send()
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        let bytes = response.bytes()
            .map_err(|e| format!("Read failed: {}", e))?;
        
        fs::write(&local_path, &bytes)
            .map_err(|e| format!("Write failed: {}", e))?;

        Ok(local_path.to_string_lossy().to_string())
    }

    pub fn save_page(&self, page: &ExtractedPage) -> Result<(), String> {
        let html_path = self.output_dir.join("page.html");
        fs::write(&html_path, &page.html)
            .map_err(|e| format!("Failed to write HTML: {}", e))?;

        let json_path = self.output_dir.join("page.json");
        let json = serde_json::to_string_pretty(page)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        fs::write(&json_path, json)
            .map_err(|e| format!("Failed to write JSON: {}", e))?;

        let components_path = self.output_dir.join("components.json");
        let comp_json = serde_json::to_string_pretty(&page.components)
            .map_err(|e| format!("Failed to serialize components: {}", e))?;
        fs::write(&components_path, comp_json)
            .map_err(|e| format!("Failed to write components: {}", e))?;

        Ok(())
    }
}

#[cfg(feature = "site_retriever")]
fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#).unwrap();
    re.replace_all(name, "_").to_string()
}

#[cfg(not(feature = "site_retriever"))]
#[allow(dead_code)]
pub struct SiteRetriever;

#[cfg(not(feature = "site_retriever"))]
#[allow(dead_code)]
impl SiteRetriever {
    pub fn new(_output_dir: std::path::PathBuf, _download_assets: bool) -> Result<Self, String> {
        Err("site_retriever feature not enabled. Rebuild with --features site_retriever".into())
    }
}