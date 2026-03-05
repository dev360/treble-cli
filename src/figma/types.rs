//! Figma API response types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── File-level response ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FileResponse {
    pub name: String,
    pub document: DocumentNode,
    #[serde(rename = "lastModified")]
    pub last_modified: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct DocumentNode {
    pub children: Vec<CanvasNode>,
}

/// A canvas (page) in the Figma file
#[derive(Debug, Deserialize)]
pub struct CanvasNode {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub node_type: String,
    #[serde(default)]
    pub children: Vec<serde_json::Value>,
}

// ── Node-level response ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct NodesResponse {
    pub nodes: HashMap<String, Option<NodeWrapper>>,
}

#[derive(Debug, Deserialize)]
pub struct NodeWrapper {
    pub document: serde_json::Value,
}

// ── Image response ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ImageResponse {
    pub images: HashMap<String, Option<String>>,
}

// ── User identity ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MeResponse {
    #[allow(dead_code)]
    pub id: String,
    pub email: String,
    pub handle: String,
}

// ── Flattened node for disk storage ─────────────────────────────────────

/// A node from the Figma tree, flattened for on-disk storage.
/// Contains visual properties needed for code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub depth: u32,

    // Geometry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,

    // Component info
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_component: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_name: Option<String>,

    // Layout
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_auto_layout: bool,
    #[serde(default)]
    pub child_count: u32,

    // Visual properties for code gen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fills: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strokes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
}

// ── Manifest for sync tracking ──────────────────────────────────────────

/// On-disk manifest at .treble/figma/manifest.json
/// Tracks file-level metadata + frame inventory for deterministic sync.
#[derive(Debug, Serialize, Deserialize)]
pub struct FigmaManifest {
    pub file_key: String,
    pub file_name: String,
    pub last_modified: String,
    pub version: String,
    pub synced_at: String,
    pub frames: Vec<FrameManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameManifestEntry {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub page_name: String,
    pub node_count: u32,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub synced_at: String,
}

const MAX_SLUG_LEN: usize = 80;

/// Create a deterministic slug from a name.
/// "Contact Form" → "contact-form", "Hero Section (v2)" → "hero-section-v2"
/// Returns "unnamed" for empty/non-alphanumeric inputs.
/// Truncates to 80 chars max.
pub fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        return "unnamed".to_string();
    }

    if slug.len() <= MAX_SLUG_LEN {
        slug
    } else {
        // Truncate and append a 6-char hash to avoid collisions
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        slug.hash(&mut hasher);
        let hash = format!("{:06x}", hasher.finish() & 0xFFFFFF);
        let truncated: String = slug.chars().take(MAX_SLUG_LEN - 7).collect();
        format!("{truncated}-{hash}")
    }
}

/// Assign unique slugs to a list of (frame_name, page_name) pairs.
/// When multiple frames share the same base slug, prefix with page slug.
/// Returns slugs in the same order as input.
pub fn assign_unique_slugs(frames: &[(String, String)]) -> Vec<String> {
    use std::collections::HashMap;

    let base_slugs: Vec<String> = frames.iter().map(|(name, _)| slugify(name)).collect();

    // Count occurrences of each base slug
    let mut slug_counts: HashMap<&str, usize> = HashMap::new();
    for slug in &base_slugs {
        *slug_counts.entry(slug.as_str()).or_insert(0) += 1;
    }

    // For collisions, prefix with page slug — then cap at MAX_SLUG_LEN
    base_slugs
        .iter()
        .enumerate()
        .map(|(i, base)| {
            if slug_counts[base.as_str()] > 1 {
                let page_slug = slugify(&frames[i].1);
                let combined = format!("{page_slug}-{base}");
                // Re-apply length cap to the combined slug
                if combined.len() <= MAX_SLUG_LEN {
                    combined
                } else {
                    slugify(&combined)
                }
            } else {
                base.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_normal() {
        assert_eq!(slugify("Contact Form"), "contact-form");
        assert_eq!(slugify("Hero Section (v2)"), "hero-section-v2");
    }

    #[test]
    fn test_slugify_empty_input() {
        assert_eq!(slugify(""), "unnamed");
        assert_eq!(slugify("🎨✨"), "unnamed");
        assert_eq!(slugify("---"), "unnamed");
    }

    #[test]
    fn test_assign_unique_slugs_no_collision() {
        let frames = vec![
            ("Contact".into(), "Mocks".into()),
            ("Home".into(), "Mocks".into()),
        ];
        assert_eq!(assign_unique_slugs(&frames), vec!["contact", "home"]);
    }

    #[test]
    fn test_assign_unique_slugs_collision() {
        let frames = vec![
            ("Contact".into(), "Mocks".into()),
            ("Contact".into(), "Wireframes".into()),
            ("Contact".into(), "Thumbnail".into()),
            ("Home".into(), "Mocks".into()),
        ];
        let slugs = assign_unique_slugs(&frames);
        assert_eq!(slugs[0], "mocks-contact");
        assert_eq!(slugs[1], "wireframes-contact");
        assert_eq!(slugs[2], "thumbnail-contact");
        assert_eq!(slugs[3], "home"); // no collision, no prefix
    }
}
