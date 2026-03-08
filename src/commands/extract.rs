//! `treble extract` — download source images from Figma IMAGE fills
//!
//! Scans each frame's nodes.json for fills with type "IMAGE", resolves their
//! imageRef hashes via the Figma file images API, downloads the source images,
//! and writes an image-map.json per frame:
//!
//!   .treble/figma/{slug}/
//!     image-map.json         — imageRef → local path + node usage
//!     assets/                — downloaded source images
//!       {first-8-chars}.png

use crate::config::{find_project_root, GlobalConfig, ProjectConfig};
use crate::figma::client::scan_image_refs;
use crate::figma::types::{FigmaManifest, FlatNode, ImageMap, ImageMapEntry};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;

pub async fn run(frame_filter: Option<String>) -> Result<()> {
    let project_root = find_project_root()?;
    let project_config = ProjectConfig::load(&project_root)?;
    let global_config = GlobalConfig::load()?;
    let client = global_config.figma_client()?;

    let file_key = &project_config.figma_file_key;
    let figma_dir = project_root.join(".treble").join("figma");

    // ── Load manifest ────────────────────────────────────────────────
    let manifest_path = figma_dir.join("manifest.json");
    if !manifest_path.exists() {
        anyhow::bail!("No manifest.json found — run `treble sync` first");
    }
    let manifest: FigmaManifest = {
        let content = std::fs::read_to_string(&manifest_path)?;
        serde_json::from_str(&content).context("Failed to parse manifest.json")?
    };

    // ── Filter frames ────────────────────────────────────────────────
    let frames: Vec<_> = manifest
        .frames
        .iter()
        .filter(|f| {
            if let Some(ref filter) = frame_filter {
                f.name.to_lowercase().contains(&filter.to_lowercase())
                    || f.slug.to_lowercase().contains(&filter.to_lowercase())
            } else {
                true
            }
        })
        .collect();

    if frames.is_empty() {
        anyhow::bail!("No frames matched the filter");
    }

    // ── Scan all frames for IMAGE refs ───────────────────────────────
    println!(
        "{} Scanning {} frame{} for image fills...",
        "→".dimmed(),
        frames.len(),
        if frames.len() == 1 { "" } else { "s" }
    );

    // Collect unique imageRefs across all frames, tracking which frame/nodes use each
    struct FrameScan {
        slug: String,
        refs: Vec<(String, Vec<crate::figma::types::ImageNodeUsage>)>,
    }

    let mut frame_scans: Vec<FrameScan> = Vec::new();
    let mut all_refs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for frame in &frames {
        let nodes_path = figma_dir.join(&frame.slug).join("nodes.json");
        if !nodes_path.exists() {
            eprintln!("  {} Skipping {} — no nodes.json", "!".yellow(), frame.slug);
            continue;
        }

        let nodes: Vec<FlatNode> = {
            let content = std::fs::read_to_string(&nodes_path)?;
            serde_json::from_str(&content)
                .context(format!("Failed to parse {}/nodes.json", frame.slug))?
        };

        let refs = scan_image_refs(&nodes);
        for (image_ref, _) in &refs {
            all_refs.insert(image_ref.clone());
        }

        frame_scans.push(FrameScan {
            slug: frame.slug.clone(),
            refs,
        });
    }

    if all_refs.is_empty() {
        println!(
            "\n{} No IMAGE fills found in any frame",
            "Done!".green().bold()
        );
        return Ok(());
    }

    println!(
        "  Found {} unique image ref{} across {} frame{}",
        all_refs.len(),
        if all_refs.len() == 1 { "" } else { "s" },
        frame_scans.len(),
        if frame_scans.len() == 1 { "" } else { "s" }
    );

    // ── Resolve imageRef hashes → CDN URLs (single API call) ────────
    println!("{} Resolving image URLs from Figma API...", "→".dimmed());
    let ref_urls: HashMap<String, String> = client.get_file_images(file_key).await?;

    let resolved: usize = all_refs
        .iter()
        .filter(|r| ref_urls.contains_key(*r))
        .count();
    println!("  Resolved {}/{} refs", resolved, all_refs.len());

    if resolved == 0 {
        println!(
            "\n{} No image URLs resolved — the imageRefs may be stale",
            "Warning".yellow().bold()
        );
        return Ok(());
    }

    // ── Download images and write image-map.json per frame ──────────
    let mut total_downloaded = 0u32;
    let mut total_skipped = 0u32;

    for scan in &frame_scans {
        if scan.refs.is_empty() {
            continue;
        }

        let frame_dir = figma_dir.join(&scan.slug);
        let assets_dir = frame_dir.join("assets");
        std::fs::create_dir_all(&assets_dir)?;

        let mut entries: Vec<ImageMapEntry> = Vec::new();

        for (image_ref, nodes) in &scan.refs {
            let short_ref = &image_ref[..image_ref.len().min(8)];
            let local_filename = format!("{short_ref}.png");
            let local_path = format!("assets/{local_filename}");
            let abs_path = assets_dir.join(&local_filename);

            // Download if not already cached
            if abs_path.exists() {
                total_skipped += 1;
            } else if let Some(url) = ref_urls.get(image_ref) {
                match client.download_image(url).await {
                    Ok(bytes) => {
                        std::fs::write(&abs_path, &bytes)?;
                        total_downloaded += 1;
                    }
                    Err(e) => {
                        eprintln!("  {} Failed to download {}: {}", "!".yellow(), short_ref, e);
                        continue;
                    }
                }
            } else {
                eprintln!("  {} No URL for imageRef {}", "!".yellow(), short_ref);
                continue;
            }

            entries.push(ImageMapEntry {
                image_ref: image_ref.clone(),
                local_path,
                nodes: nodes.clone(),
            });
        }

        // Write image-map.json
        let image_map = ImageMap {
            file_key: file_key.clone(),
            extracted_at: chrono::Utc::now().to_rfc3339(),
            entries,
        };

        let map_json = serde_json::to_string_pretty(&image_map)?;
        std::fs::write(frame_dir.join("image-map.json"), &map_json)?;
    }

    // ── Summary ──────────────────────────────────────────────────────
    println!(
        "\n{} Extracted {} image{} ({} cached, {} downloaded)",
        "Done!".green().bold(),
        total_downloaded + total_skipped,
        if total_downloaded + total_skipped == 1 {
            ""
        } else {
            "s"
        },
        total_skipped,
        total_downloaded,
    );

    Ok(())
}
