//! `treble show <node>` — render a specific Figma node and save to disk
//!
//! Takes a node name or ID, calls the Figma images API to render it,
//! saves the screenshot to .treble/figma/snapshots/{name}.png.
//!
//! This is how Claude (or a human) can "look at" any layer in the design.

use crate::config::{find_project_root, GlobalConfig, ProjectConfig};
use crate::figma::client::FigmaClient;
use crate::figma::types::{FigmaManifest, FlatNode, slugify};
use anyhow::{Context, Result};
use colored::Colorize;

pub async fn run(node_query: String, frame_name: Option<String>, scale: f64) -> Result<()> {
    let project_root = find_project_root()?;
    let project_config = ProjectConfig::load(&project_root)?;
    let global_config = GlobalConfig::load()?;
    let token = global_config.require_figma_token()?;
    let client = FigmaClient::new(token);

    let file_key = &project_config.figma_file_key;
    let figma_dir = project_root.join(".treble").join("figma");

    // Load manifest
    let manifest: FigmaManifest = serde_json::from_str(
        &std::fs::read_to_string(figma_dir.join("manifest.json"))
            .context("No synced data. Run `treble sync` first.")?,
    )?;

    // Resolve node ID from the query (could be name or ID)
    let (node_id, node_name, frame_slug) =
        resolve_node(&figma_dir, &manifest, &node_query, frame_name.as_deref())?;

    println!(
        "Rendering {} ({})...",
        node_name.bold(),
        node_id.dimmed()
    );

    // Call Figma images API
    let images = client
        .get_images(file_key, &[node_id.as_str()], scale)
        .await
        .context("Failed to get image from Figma API")?;

    let url = images
        .get(node_id.as_str())
        .and_then(|v| v.as_ref())
        .context("Figma returned no image for this node")?;

    let bytes = client.download_image(url).await?;

    // Save to .treble/figma/snapshots/ (or under the frame dir)
    let output_path = if let Some(ref slug) = frame_slug {
        let dir = figma_dir.join(slug).join("snapshots");
        std::fs::create_dir_all(&dir)?;
        dir.join(format!("{}.png", slugify(&node_name)))
    } else {
        let dir = figma_dir.join("snapshots");
        std::fs::create_dir_all(&dir)?;
        dir.join(format!("{}.png", slugify(&node_name)))
    };

    std::fs::write(&output_path, &bytes)?;

    let relative_path = output_path
        .strip_prefix(&project_root)
        .unwrap_or(&output_path);

    println!(
        "{} Saved to {}",
        "Done!".green().bold(),
        relative_path.display()
    );
    println!("  Size: {} bytes", bytes.len());
    println!("  Scale: {}x", scale);

    Ok(())
}

/// Resolve a node query (name or ID) to (node_id, node_name, frame_slug).
/// If the query looks like a Figma node ID (contains ":"), use it directly.
/// Otherwise, search nodes.json files to find a matching name.
fn resolve_node(
    figma_dir: &std::path::Path,
    manifest: &FigmaManifest,
    query: &str,
    frame_filter: Option<&str>,
) -> Result<(String, String, Option<String>)> {
    // If it looks like a node ID (e.g. "55:1234"), use directly
    if query.contains(':') && query.chars().all(|c| c.is_ascii_digit() || c == ':') {
        return Ok((query.to_string(), query.to_string(), None));
    }

    // Search through synced nodes.json files
    let frames_to_search: Vec<_> = if let Some(filter) = frame_filter {
        manifest
            .frames
            .iter()
            .filter(|f| f.name.to_lowercase().contains(&filter.to_lowercase()))
            .collect()
    } else {
        manifest.frames.iter().collect()
    };

    if frames_to_search.is_empty() {
        anyhow::bail!("No frames to search. Specify --frame or run `treble sync`.");
    }

    let query_lower = query.to_lowercase();

    for frame in &frames_to_search {
        let nodes_path = figma_dir.join(&frame.slug).join("nodes.json");
        if !nodes_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&nodes_path)?;
        let nodes: Vec<FlatNode> = serde_json::from_str(&content)?;

        // Exact name match first
        if let Some(node) = nodes.iter().find(|n| n.name == query) {
            return Ok((node.id.clone(), node.name.clone(), Some(frame.slug.clone())));
        }

        // Case-insensitive contains
        if let Some(node) = nodes
            .iter()
            .find(|n| n.name.to_lowercase().contains(&query_lower))
        {
            return Ok((node.id.clone(), node.name.clone(), Some(frame.slug.clone())));
        }
    }

    // List some available names to help
    let mut available = Vec::new();
    for frame in &frames_to_search {
        let nodes_path = figma_dir.join(&frame.slug).join("nodes.json");
        if !nodes_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&nodes_path)?;
        let nodes: Vec<FlatNode> = serde_json::from_str(&content)?;
        // Show depth-0 and depth-1 names
        for node in nodes.iter().filter(|n| n.depth <= 1) {
            available.push(format!("  {} ({})", node.name, frame.name));
        }
    }

    anyhow::bail!(
        "No node matching \"{query}\". Top-level layers:\n{}",
        available.join("\n")
    );
}
