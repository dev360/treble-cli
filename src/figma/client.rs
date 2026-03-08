//! Figma REST API client

use super::types::*;
use anyhow::{Context, Result};
use std::collections::HashMap;

const FIGMA_API_BASE: &str = "https://api.figma.com/v1";

pub struct FigmaClient {
    token: String,
    is_oauth: bool,
    http: reqwest::Client,
}

impl FigmaClient {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
            is_oauth: false,
            http: reqwest::Client::new(),
        }
    }

    pub fn new_oauth(token: &str) -> Self {
        Self {
            token: token.to_string(),
            is_oauth: true,
            http: reqwest::Client::new(),
        }
    }

    /// Check a Figma API response for common errors (403, 404, 429, other failures).
    /// Returns Ok(response) if status is success.
    async fn check_response(resp: reqwest::Response, context: &str) -> Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        // Read the body for better diagnostics
        let body = resp.text().await.unwrap_or_default();
        match status.as_u16() {
            403 => anyhow::bail!(
                "{context}: token is invalid, expired, or lacks required scope\n  Response: {body}"
            ),
            404 => anyhow::bail!(
                "{context}: not found — check the URL or token permissions\n  Response: {body}"
            ),
            429 => anyhow::bail!(
                "Figma API rate limited — wait a minute and retry\n  Response: {body}"
            ),
            _ => anyhow::bail!("{context}: Figma API error ({status})\n  Response: {body}"),
        }
    }

    /// Send a GET request to the Figma API with automatic retry on 429.
    async fn figma_get_with_retry(&self, url: &str, context: &str) -> Result<reqwest::Response> {
        let delays = [0, 5, 15, 30]; // seconds to wait before each attempt
        for (i, delay) in delays.iter().enumerate() {
            if *delay > 0 {
                eprintln!(
                    "  Rate limited — retrying in {}s (attempt {}/{})",
                    delay,
                    i + 1,
                    delays.len()
                );
                tokio::time::sleep(std::time::Duration::from_secs(*delay)).await;
            }
            let mut req = self.http.get(url);
            if self.is_oauth {
                req = req.header("Authorization", format!("Bearer {}", self.token));
            } else {
                req = req.header("X-Figma-Token", &self.token);
            }
            let resp = req.send().await.context("Failed to reach Figma API")?;

            if resp.status().as_u16() == 429 && i < delays.len() - 1 {
                // Consume body so connection is released, then retry
                let _ = resp.text().await;
                continue;
            }
            return Self::check_response(resp, context).await;
        }
        unreachable!()
    }

    /// GET /v1/me — validate token and return user identity
    pub async fn me(&self) -> Result<MeResponse> {
        let resp = self
            .figma_get_with_retry(&format!("{FIGMA_API_BASE}/me"), "/v1/me")
            .await?;
        resp.json::<MeResponse>()
            .await
            .context("Failed to parse /v1/me response")
    }

    /// GET /v1/files/:key — file metadata + document tree
    pub async fn get_file(&self, file_key: &str) -> Result<FileResponse> {
        let url = format!("{FIGMA_API_BASE}/files/{file_key}?depth=2");
        let resp = self
            .figma_get_with_retry(&url, &format!("files/{file_key}"))
            .await?;
        resp.json::<FileResponse>()
            .await
            .context("Failed to parse file response")
    }

    /// GET /v1/files/:key/nodes?ids=... — full node tree for specific nodes
    pub async fn get_nodes(&self, file_key: &str, node_ids: &[&str]) -> Result<NodesResponse> {
        let ids = node_ids.join(",");
        let url = format!("{FIGMA_API_BASE}/files/{file_key}/nodes?ids={ids}");
        let resp = self.figma_get_with_retry(&url, "nodes").await?;
        resp.json::<NodesResponse>()
            .await
            .context("Failed to parse nodes response")
    }

    /// GET /v1/images/:key?ids=...&format=png — render node images
    pub async fn get_images(
        &self,
        file_key: &str,
        node_ids: &[&str],
        scale: f64,
    ) -> Result<HashMap<String, Option<String>>> {
        let ids = node_ids.join(",");
        let url = format!("{FIGMA_API_BASE}/images/{file_key}?ids={ids}&format=png&scale={scale}");
        let resp = self.figma_get_with_retry(&url, "images").await?;
        let image_resp: ImageResponse = resp
            .json()
            .await
            .context("Failed to parse image response")?;
        Ok(image_resp.images)
    }

    /// GET /v1/files/:key/images — resolve all imageRef hashes to CDN download URLs.
    /// Returns a map of imageRef → URL. This is the SOURCE image endpoint, distinct
    /// from GET /v1/images/:key which renders composited screenshots.
    pub async fn get_file_images(&self, file_key: &str) -> Result<HashMap<String, String>> {
        let url = format!("{FIGMA_API_BASE}/files/{file_key}/images");
        let resp = self
            .figma_get_with_retry(&url, &format!("files/{file_key}/images"))
            .await?;
        let file_images: FileImagesResponse = resp
            .json()
            .await
            .context("Failed to parse file images response")?;
        Ok(file_images.meta.images)
    }

    /// Download an image URL to bytes
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .context("Failed to download image")?;

        if !resp.status().is_success() {
            anyhow::bail!("Image download failed ({})", resp.status());
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .context("Failed to read image bytes")
    }
}

/// Flatten a Figma node tree (serde_json::Value) into a Vec<FlatNode>.
/// Recursively walks the tree, extracting visual properties at each level.
pub fn flatten_node_tree(
    node: &serde_json::Value,
    parent_id: Option<&str>,
    depth: u32,
) -> Vec<FlatNode> {
    let mut result = Vec::new();

    let id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");

    // Extract bounding box
    let bbox = node.get("absoluteBoundingBox");
    let x = bbox.and_then(|b| b.get("x")).and_then(|v| v.as_f64());
    let y = bbox.and_then(|b| b.get("y")).and_then(|v| v.as_f64());
    let width = bbox.and_then(|b| b.get("width")).and_then(|v| v.as_f64());
    let height = bbox.and_then(|b| b.get("height")).and_then(|v| v.as_f64());

    // Component info
    let is_component = node_type == "COMPONENT" || node_type == "COMPONENT_SET";
    let component_name = if is_component {
        Some(name.to_string())
    } else {
        None
    };

    // Auto layout
    let has_auto_layout = node
        .get("layoutMode")
        .and_then(|v| v.as_str())
        .map(|m| m == "HORIZONTAL" || m == "VERTICAL")
        .unwrap_or(false);

    let children = node
        .get("children")
        .and_then(|v| v.as_array())
        .map(|a| a.len() as u32)
        .unwrap_or(0);

    // Visual properties
    let characters = node
        .get("characters")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let fills = node.get("fills").cloned();
    let strokes = node.get("strokes").cloned();
    let effects = node.get("effects").cloned();
    let corner_radius = node.get("cornerRadius").and_then(|v| v.as_f64());

    // Typography
    let style = node.get("style");
    let font_family = style
        .and_then(|s| s.get("fontFamily"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let font_size = style
        .and_then(|s| s.get("fontSize"))
        .and_then(|v| v.as_f64());
    let font_weight = style
        .and_then(|s| s.get("fontWeight"))
        .and_then(|v| v.as_f64());
    let line_height = style.and_then(|s| s.get("lineHeightPx")).cloned();

    // Layout
    let layout_mode = node
        .get("layoutMode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let padding_left = node.get("paddingLeft").and_then(|v| v.as_f64());
    let padding_right = node.get("paddingRight").and_then(|v| v.as_f64());
    let padding_top = node.get("paddingTop").and_then(|v| v.as_f64());
    let padding_bottom = node.get("paddingBottom").and_then(|v| v.as_f64());
    let item_spacing = node.get("itemSpacing").and_then(|v| v.as_f64());
    let opacity = node.get("opacity").and_then(|v| v.as_f64());

    let flat = FlatNode {
        id: id.to_string(),
        name: name.to_string(),
        node_type: node_type.to_string(),
        parent_id: parent_id.map(|s| s.to_string()),
        depth,
        x,
        y,
        width,
        height,
        is_component,
        component_name,
        has_auto_layout,
        child_count: children,
        characters,
        fills,
        strokes,
        effects,
        corner_radius,
        font_family,
        font_size,
        font_weight,
        line_height,
        layout_mode,
        padding_left,
        padding_right,
        padding_top,
        padding_bottom,
        item_spacing,
        opacity,
    };

    result.push(flat);

    // Recurse into children
    if let Some(child_array) = node.get("children").and_then(|v| v.as_array()) {
        for child in child_array {
            result.extend(flatten_node_tree(child, Some(id), depth + 1));
        }
    }

    result
}

/// Scan a list of FlatNodes for IMAGE fills, returning (imageRef, nodeId, nodeName, width, height).
/// Deduplicates by imageRef — multiple nodes may share the same source image.
pub fn scan_image_refs(nodes: &[FlatNode]) -> Vec<(String, Vec<ImageNodeUsage>)> {
    use std::collections::HashMap;
    let mut ref_map: HashMap<String, Vec<ImageNodeUsage>> = HashMap::new();

    for node in nodes {
        if let Some(ref fills) = node.fills {
            if let Some(arr) = fills.as_array() {
                for fill in arr {
                    let fill_type = fill.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if fill_type == "IMAGE" {
                        if let Some(image_ref) = fill.get("imageRef").and_then(|v| v.as_str()) {
                            ref_map.entry(image_ref.to_string()).or_default().push(
                                ImageNodeUsage {
                                    node_id: node.id.clone(),
                                    node_name: node.name.clone(),
                                    width: node.width,
                                    height: node.height,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    ref_map.into_iter().collect()
}
