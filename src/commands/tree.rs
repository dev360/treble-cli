//! `treble tree <frame>` — print the layer outline for a synced frame
//!
//! Reads .treble/figma/{slug}/nodes.json and prints an indented tree
//! showing the Figma layer hierarchy with types, sizes, and key properties.
//!
//! --root <nodeId|name>  Show only the subtree rooted at a specific node.
//! --json                Output compact JSON for agent consumption.

use crate::config::find_project_root;
use crate::figma::types::{FigmaManifest, FlatNode};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;

pub fn run(
    frame_name: String,
    max_depth: Option<u32>,
    verbose: bool,
    root_filter: Option<String>,
    json_output: bool,
) -> Result<()> {
    let project_root = find_project_root()?;
    let figma_dir = project_root.join(".treble").join("figma");

    // Load manifest to resolve frame name → slug
    let manifest_path = figma_dir.join("manifest.json");
    let manifest: FigmaManifest = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .context("No synced data. Run `treble sync` first.")?,
    )?;

    // Find the frame (fuzzy match on name)
    let entry = manifest
        .frames
        .iter()
        .find(|f| f.name.to_lowercase().contains(&frame_name.to_lowercase()))
        .with_context(|| {
            let available: Vec<&str> = manifest.frames.iter().map(|f| f.name.as_str()).collect();
            format!(
                "No frame matching \"{frame_name}\". Available: {}",
                available.join(", ")
            )
        })?;

    let slug = &entry.slug;

    // Load nodes
    let nodes_path = figma_dir.join(slug).join("nodes.json");
    let nodes: Vec<FlatNode> = serde_json::from_str(
        &std::fs::read_to_string(&nodes_path)
            .context(format!(
                "No nodes.json for frame \"{}\". Run `treble sync`.",
                entry.name
            ))?,
    )?;

    // If --root is specified, find the root node and filter to its subtree
    let (display_nodes, root_depth_offset) = if let Some(ref root_query) = root_filter {
        let root_node = find_node_by_query(&nodes, root_query).with_context(|| {
            format!("No node matching \"{}\" in frame \"{}\"", root_query, entry.name)
        })?;
        let subtree = extract_subtree(&nodes, &root_node.id);
        let offset = root_node.depth;
        (subtree, offset)
    } else {
        (nodes.clone(), 0)
    };

    if json_output {
        return print_json(&display_nodes, root_depth_offset, max_depth, entry);
    }

    // Print header
    println!(
        "{} \"{}\" ({}) — {} nodes",
        "Frame:".bold(),
        entry.name,
        entry.id,
        display_nodes.len()
    );
    if let Some(ref root_query) = root_filter {
        println!("  Root: \"{}\"", root_query);
    }
    if let (Some(w), Some(h)) = (entry.width, entry.height) {
        println!("  Size: {}x{}", w, h);
    }

    // Check for section screenshots
    let sections_dir = figma_dir.join(slug).join("sections");
    if sections_dir.is_dir() && root_filter.is_none() {
        let section_files: Vec<String> = std::fs::read_dir(&sections_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "png"))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        if !section_files.is_empty() {
            println!("  Section screenshots: {}", section_files.join(", "));
        }
    }
    println!();

    // Print tree
    for node in &display_nodes {
        let relative_depth = node.depth - root_depth_offset;

        if let Some(max) = max_depth {
            if relative_depth > max {
                continue;
            }
        }

        let indent = "  ".repeat(relative_depth as usize);
        let type_badge = format_type_badge(&node.node_type);
        let size_str = match (node.width, node.height) {
            (Some(w), Some(h)) => format!(" [{}x{}]", w.round(), h.round()),
            _ => String::new(),
        };

        let name_display = if node.name.chars().count() > 40 {
            let truncated: String = node.name.chars().take(37).collect();
            format!("{truncated}...")
        } else {
            node.name.clone()
        };

        // Node ID for --root usage
        let id_hint = format!(" {}", node.id.dimmed());

        print!("{indent}{type_badge} {name_display}{size_str}{id_hint}");

        if node.is_component {
            print!(" {}", "(component)".cyan());
        }
        if node.has_auto_layout {
            let dir = node.layout_mode.as_deref().unwrap_or("?");
            print!(" {}", format!("[{dir}]").dimmed());
        }
        if let Some(ref chars) = node.characters {
            let preview = if chars.chars().count() > 30 {
                let truncated: String = chars.chars().take(27).collect();
                format!("\"{truncated}...\"")
            } else {
                format!("\"{}\"", chars)
            };
            print!(" {}", preview.green());
        }
        if node.child_count > 0 {
            print!(" {}", format!("({} children)", node.child_count).dimmed());
        }

        println!();

        if verbose {
            print_verbose_props(node, &indent);
        }
    }

    // Footer
    println!();
    print_type_summary(&display_nodes);

    Ok(())
}

/// Find a node by name (fuzzy) or ID (exact).
fn find_node_by_query<'a>(nodes: &'a [FlatNode], query: &str) -> Option<&'a FlatNode> {
    // Try exact ID match first
    if let Some(node) = nodes.iter().find(|n| n.id == query) {
        return Some(node);
    }
    // Fuzzy name match (case-insensitive)
    let query_lower = query.to_lowercase();
    nodes
        .iter()
        .find(|n| n.name.to_lowercase().contains(&query_lower))
}

/// Extract the subtree rooted at root_id (inclusive).
fn extract_subtree(nodes: &[FlatNode], root_id: &str) -> Vec<FlatNode> {
    let mut included_ids: HashSet<&str> = HashSet::new();
    included_ids.insert(root_id);

    // Walk nodes in order; include any node whose parent is already included
    let mut result = Vec::new();
    for node in nodes {
        if node.id == root_id || node.parent_id.as_deref().is_some_and(|pid| included_ids.contains(pid)) {
            included_ids.insert(&node.id);
            result.push(node.clone());
        }
    }
    result
}

/// Output compact JSON for agent/LLM consumption.
fn print_json(
    nodes: &[FlatNode],
    root_depth_offset: u32,
    max_depth: Option<u32>,
    entry: &crate::figma::types::FrameManifestEntry,
) -> Result<()> {
    use serde_json::json;

    let filtered: Vec<serde_json::Value> = nodes
        .iter()
        .filter(|n| {
            if let Some(max) = max_depth {
                n.depth - root_depth_offset <= max
            } else {
                true
            }
        })
        .map(|n| {
            let mut obj = json!({
                "id": n.id,
                "name": n.name,
                "type": n.node_type,
                "depth": n.depth - root_depth_offset,
            });
            let m = obj.as_object_mut().unwrap();

            if let (Some(w), Some(h)) = (n.width, n.height) {
                m.insert("width".into(), json!(w.round()));
                m.insert("height".into(), json!(h.round()));
            }
            if let (Some(x), Some(y)) = (n.x, n.y) {
                m.insert("x".into(), json!(x.round()));
                m.insert("y".into(), json!(y.round()));
            }
            if n.child_count > 0 {
                m.insert("children".into(), json!(n.child_count));
            }
            if let Some(ref chars) = n.characters {
                m.insert("text".into(), json!(chars));
            }
            if n.is_component {
                m.insert("component".into(), json!(true));
            }
            if n.has_auto_layout {
                m.insert("layout".into(), json!(n.layout_mode));
            }
            // Compact fill colors (hex only, skip gradients/images for brevity)
            if let Some(ref fills) = n.fills {
                if let Some(arr) = fills.as_array() {
                    let colors: Vec<String> = arr
                        .iter()
                        .filter(|f| f.get("type").and_then(|t| t.as_str()) == Some("SOLID"))
                        .filter_map(|f| {
                            let c = f.get("color")?;
                            let r = (c.get("r")?.as_f64()? * 255.0) as u8;
                            let g = (c.get("g")?.as_f64()? * 255.0) as u8;
                            let b = (c.get("b")?.as_f64()? * 255.0) as u8;
                            Some(format!("#{r:02x}{g:02x}{b:02x}"))
                        })
                        .collect();
                    if !colors.is_empty() {
                        m.insert("fills".into(), json!(colors));
                    }
                }
            }
            if let Some(ref family) = n.font_family {
                let mut font = json!({ "family": family });
                if let Some(size) = n.font_size {
                    font.as_object_mut().unwrap().insert("size".into(), json!(size));
                }
                if let Some(weight) = n.font_weight {
                    font.as_object_mut().unwrap().insert("weight".into(), json!(weight));
                }
                m.insert("font".into(), font);
            }
            if let Some(r) = n.corner_radius {
                if r > 0.0 {
                    m.insert("radius".into(), json!(r));
                }
            }

            obj
        })
        .collect();

    let output = json!({
        "frame": entry.name,
        "frameId": entry.id,
        "nodeCount": filtered.len(),
        "nodes": filtered,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn format_type_badge(node_type: &str) -> String {
    match node_type {
        "FRAME" => "FRAME".blue().to_string(),
        "TEXT" => "TEXT".green().to_string(),
        "RECTANGLE" => "RECT".yellow().to_string(),
        "VECTOR" => "VEC".magenta().to_string(),
        "COMPONENT" => "COMP".cyan().bold().to_string(),
        "COMPONENT_SET" => "CSET".cyan().bold().to_string(),
        "INSTANCE" => "INST".cyan().to_string(),
        "GROUP" => "GRP".dimmed().to_string(),
        "ELLIPSE" => "ELLI".yellow().to_string(),
        "LINE" => "LINE".dimmed().to_string(),
        "BOOLEAN_OPERATION" => "BOOL".dimmed().to_string(),
        other => other.dimmed().to_string(),
    }
}

fn print_verbose_props(node: &FlatNode, indent: &str) {
    let sub_indent = format!("{indent}  ");

    if let Some(ref family) = node.font_family {
        let size = node.font_size.map(|s| format!(" {s}px")).unwrap_or_default();
        let weight = node.font_weight.map(|w| format!(" w{w}")).unwrap_or_default();
        println!(
            "{sub_indent}{}",
            format!("font: {family}{size}{weight}").dimmed()
        );
    }

    if let Some(ref fills) = node.fills {
        if let Some(fill_arr) = fills.as_array() {
            for fill in fill_arr {
                if fill.get("type").and_then(|t| t.as_str()) == Some("SOLID") {
                    if let Some(color) = fill.get("color") {
                        let r =
                            (color.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0) * 255.0) as u8;
                        let g =
                            (color.get("g").and_then(|v| v.as_f64()).unwrap_or(0.0) * 255.0) as u8;
                        let b =
                            (color.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0) * 255.0) as u8;
                        println!(
                            "{sub_indent}{}",
                            format!("fill: #{r:02x}{g:02x}{b:02x}").dimmed()
                        );
                    }
                }
                if fill.get("type").and_then(|t| t.as_str()) == Some("IMAGE") {
                    println!("{sub_indent}{}", "fill: IMAGE".dimmed());
                }
            }
        }
    }

    if node.has_auto_layout {
        let parts: Vec<String> = [
            node.padding_top.map(|v| format!("pt:{v}")),
            node.padding_bottom.map(|v| format!("pb:{v}")),
            node.padding_left.map(|v| format!("pl:{v}")),
            node.padding_right.map(|v| format!("pr:{v}")),
            node.item_spacing.map(|v| format!("gap:{v}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !parts.is_empty() {
            println!(
                "{sub_indent}{}",
                format!("layout: {}", parts.join(" ")).dimmed()
            );
        }
    }

    if let Some(r) = node.corner_radius {
        if r > 0.0 {
            println!("{sub_indent}{}", format!("radius: {r}").dimmed());
        }
    }
}

fn print_type_summary(nodes: &[FlatNode]) {
    let mut counts = std::collections::HashMap::new();
    for node in nodes {
        *counts.entry(node.node_type.as_str()).or_insert(0u32) += 1;
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let summary: Vec<String> = sorted
        .iter()
        .map(|(t, c)| format!("{t}: {c}"))
        .collect();
    println!("{}", format!("Summary: {}", summary.join(", ")).dimmed());
}
