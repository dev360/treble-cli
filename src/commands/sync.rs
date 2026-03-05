//! `treble sync` — deterministic Figma-to-disk synchronization
//!
//! Pulls Figma file data and writes a git-friendly mirror to .treble/figma/:
//!   manifest.json           — file metadata + frame inventory
//!   {frame-slug}/           — one directory per frame
//!     reference.png         — frame screenshot (full page)
//!     nodes.json            — full flattened node tree with visual properties
//!     sections/             — depth-1 section screenshots
//!       {section-slug}.png  — individual section rendered by Figma
//!
//! Deleted frames → their directories get removed.
//! Changed frames → nodes.json + screenshots updated.
//! Everything is visible via `git diff .treble/`.

use crate::config::{find_project_root, GlobalConfig, ProjectConfig};
use crate::figma::client::{flatten_node_tree, FigmaClient};
use crate::figma::types::{
    assign_unique_slugs, slugify, FigmaManifest, FlatNode, FrameManifestEntry,
};
use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;

/// A frame candidate collected from the Figma file.
struct FrameInfo {
    id: String,
    name: String,
    page_name: String,
    short_id: String, // e.g. "f01", "f02"
}

pub async fn run(
    frame_filter: Option<String>,
    page_filter: Option<String>,
    force: bool,
    interactive: bool,
) -> Result<()> {
    let project_root = find_project_root()?;
    let project_config = ProjectConfig::load(&project_root)?;
    let global_config = GlobalConfig::load()?;
    let token = global_config.require_figma_token()?;
    let client = FigmaClient::new(token);

    let file_key = &project_config.figma_file_key;
    let figma_dir = project_root.join(".treble").join("figma");
    std::fs::create_dir_all(&figma_dir)?;

    // ── Step 1: Fetch file info ─────────────────────────────────────────
    println!("{}", "Fetching Figma file...".dimmed());
    let file = client.get_file(file_key).await?;
    println!(
        "  {} \"{}\" (version: {})",
        "→".dimmed(),
        file.name.bold(),
        file.version
    );

    // ── Step 2: Enumerate ALL frames ────────────────────────────────────
    let mut all_frames: Vec<FrameInfo> = Vec::new();
    let mut counter = 0u32;

    for page in &file.document.children {
        for child in &page.children {
            let frame_id = child
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let frame_name = child
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if frame_id.is_empty() || frame_name.is_empty() {
                continue;
            }

            let short_id = format!("f{:02}", counter);
            counter += 1;

            all_frames.push(FrameInfo {
                id: frame_id,
                name: frame_name,
                page_name: page.name.clone(),
                short_id,
            });
        }
    }

    // ── Step 2b: Filter frames ──────────────────────────────────────────
    let selected_indices: Vec<usize> = if interactive {
        interactive_select(&file.document.children, &all_frames)?
    } else {
        // Apply CLI filters
        all_frames
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                if let Some(ref filter) = page_filter {
                    if !f.page_name.to_lowercase().contains(&filter.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref filter) = frame_filter {
                    if !f.name.to_lowercase().contains(&filter.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .map(|(i, _)| i)
            .collect()
    };

    if selected_indices.is_empty() {
        if interactive {
            println!("{}", "Cancelled.".dimmed());
            return Ok(());
        }
        anyhow::bail!("No frames matched the filter");
    }

    // Build the filtered frames list
    let frames: Vec<(&FrameInfo, String)> = {
        // Compute slugs for ALL frames first (so collision detection is global)
        let all_slug_inputs: Vec<(String, String)> = all_frames
            .iter()
            .map(|f| (f.name.clone(), f.page_name.clone()))
            .collect();
        let all_slugs = assign_unique_slugs(&all_slug_inputs);

        selected_indices
            .iter()
            .map(|&i| (&all_frames[i], all_slugs[i].clone()))
            .collect()
    };

    let is_filtered = frame_filter.is_some() || page_filter.is_some() || interactive;
    println!("  {} frames to sync\n", frames.len());

    // ── Step 3: Load existing manifest for diff + incremental sync ─────
    let manifest_path = figma_dir.join("manifest.json");
    let old_manifest: Option<FigmaManifest> = if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)?;
        serde_json::from_str(&content).ok()
    } else {
        None
    };
    let old_slugs: HashSet<String> = old_manifest
        .as_ref()
        .map(|m| m.frames.iter().map(|f| f.slug.clone()).collect())
        .unwrap_or_default();

    // For incremental sync: set of frame IDs already synced (with nodes.json on disk)
    let already_synced: HashSet<String> = if force {
        HashSet::new()
    } else {
        old_manifest
            .as_ref()
            .map(|m| {
                m.frames
                    .iter()
                    .filter(|f| figma_dir.join(&f.slug).join("nodes.json").exists())
                    .map(|f| f.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    };

    // ── Step 4: Filter to frames that need syncing ─────────────────────
    let mut manifest_entries: Vec<FrameManifestEntry> = Vec::new();
    let mut new_slugs: HashSet<String> = HashSet::new();
    let mut skipped = 0u32;

    // Separate into needs-sync vs already-synced
    let mut to_sync: Vec<(&FrameInfo, String)> = Vec::new();
    for (frame, slug) in &frames {
        new_slugs.insert(slug.clone());
        if already_synced.contains(&frame.id) {
            if let Some(ref old) = old_manifest {
                if let Some(entry) = old.frames.iter().find(|f| f.id == frame.id) {
                    manifest_entries.push(FrameManifestEntry {
                        slug: slug.clone(),
                        ..entry.clone()
                    });
                }
            }
            skipped += 1;
        } else {
            to_sync.push((frame, slug.clone()));
        }
    }

    if to_sync.is_empty() {
        println!(
            "\n{} All {} frames already synced (use --force to re-sync)",
            "Done!".green().bold(),
            skipped
        );
        return Ok(());
    }

    // ── Step 4b: Batch fetch node trees (chunks of 30) ───────────────
    const NODE_BATCH_SIZE: usize = 30;
    let total_to_sync = to_sync.len();
    println!(
        "  Fetching {} frame{} ({} batches)...",
        total_to_sync,
        if total_to_sync == 1 { "" } else { "s" },
        (total_to_sync + NODE_BATCH_SIZE - 1) / NODE_BATCH_SIZE
    );

    let pb = ProgressBar::new(total_to_sync as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:30}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    // Process in batches
    for chunk in to_sync.chunks(NODE_BATCH_SIZE) {
        // Batch node fetch
        let batch_ids: Vec<&str> = chunk.iter().map(|(f, _)| f.id.as_str()).collect();
        pb.set_message(format!("fetching {} nodes...", batch_ids.len()));

        let nodes_resp = client.get_nodes(file_key, &batch_ids).await?;

        // Collect all image IDs needed for this batch (frames + sections)
        let mut all_image_ids: Vec<String> = Vec::new();
        // Store per-frame data for writing after image fetch
        struct FrameData {
            flat_nodes: Vec<FlatNode>,
            sections: Vec<SectionInfo>,
            frame_width: Option<f64>,
            frame_height: Option<f64>,
        }
        let mut frame_data: Vec<Option<FrameData>> = Vec::new();

        for (frame, slug) in chunk {
            let frame_dir = figma_dir.join(slug);
            if let Err(e) = std::fs::create_dir_all(&frame_dir) {
                pb.println(format!(
                    "  {} Skipping {} ({}): {}",
                    "!".yellow(),
                    truncate_display(&frame.name, 30),
                    slug,
                    e
                ));
                frame_data.push(None);
                pb.inc(1);
                continue;
            }

            let doc = nodes_resp
                .nodes
                .get(&frame.id)
                .and_then(|n| n.as_ref())
                .map(|n| &n.document);

            if let Some(doc) = doc {
                let flat_nodes = flatten_node_tree(doc, None, 0);
                let frame_width = flat_nodes.first().and_then(|n| n.width);
                let frame_height = flat_nodes.first().and_then(|n| n.height);

                // Write nodes.json immediately
                let nodes_json = serde_json::to_string_pretty(&flat_nodes)?;
                std::fs::write(frame_dir.join("nodes.json"), &nodes_json)?;

                let sections = find_sections(&flat_nodes, frame_width);

                // Collect image IDs
                all_image_ids.push(frame.id.clone());
                all_image_ids.extend(sections.iter().map(|s| s.id.clone()));

                frame_data.push(Some(FrameData {
                    flat_nodes,
                    sections,
                    frame_width,
                    frame_height,
                }));
            } else {
                pb.println(format!(
                    "  {} No node data for {}",
                    "!".yellow(),
                    truncate_display(&frame.name, 40)
                ));
                frame_data.push(None);
                pb.inc(1);
            }
        }

        // Batch image fetch (all frames + sections in one call)
        let image_urls = if !all_image_ids.is_empty() {
            pb.set_message("fetching images...");
            let img_refs: Vec<&str> = all_image_ids.iter().map(|s| s.as_str()).collect();
            client.get_images(file_key, &img_refs, 2.0).await.ok()
        } else {
            None
        };

        // Write images + manifest entries
        for (idx, (frame, slug)) in chunk.iter().enumerate() {
            let data = match &frame_data[idx] {
                Some(d) => d,
                None => continue,
            };

            let frame_dir = figma_dir.join(slug);

            // Download frame reference image
            if let Some(ref images) = image_urls {
                if let Some(Some(url)) = images.get(frame.id.as_str()) {
                    match client.download_image(url).await {
                        Ok(bytes) => {
                            std::fs::write(frame_dir.join("reference.png"), &bytes)?;
                        }
                        Err(e) => {
                            pb.println(format!(
                                "  {} Failed to download image: {}",
                                "!".yellow(),
                                e
                            ));
                        }
                    }
                }

                // Download section images
                if !data.sections.is_empty() {
                    let sections_dir = frame_dir.join("sections");
                    std::fs::create_dir_all(&sections_dir)?;
                    for section in &data.sections {
                        if let Some(Some(url)) = images.get(section.id.as_str()) {
                            match client.download_image(url).await {
                                Ok(bytes) => {
                                    let filename = format!("{}.png", slugify(&section.name));
                                    std::fs::write(sections_dir.join(&filename), &bytes)?;
                                }
                                Err(e) => {
                                    pb.println(format!(
                                        "  {} Failed to download section {}: {}",
                                        "!".yellow(),
                                        section.name,
                                        e
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            let section_count = data.sections.len();
            manifest_entries.push(FrameManifestEntry {
                id: frame.id.clone(),
                name: frame.name.clone(),
                slug: slug.clone(),
                page_name: frame.page_name.clone(),
                node_count: data.flat_nodes.len() as u32,
                width: data.frame_width,
                height: data.frame_height,
                synced_at: chrono::Utc::now().to_rfc3339(),
            });

            pb.println(format!(
                "  {} {} ({} nodes, {} sections)",
                "→".dimmed(),
                truncate_display(&frame.name, 60),
                data.flat_nodes.len(),
                section_count,
            ));
            pb.inc(1);
        }
    }

    pb.finish_and_clear();

    // ── Step 5: Delete orphaned frame directories ───────────────────────
    let orphaned: Vec<String> = if !is_filtered {
        old_slugs.difference(&new_slugs).cloned().collect()
    } else {
        Vec::new()
    };
    for slug in &orphaned {
        let orphan_dir = figma_dir.join(slug);
        if orphan_dir.is_dir() {
            std::fs::remove_dir_all(&orphan_dir)?;
            println!("  {} Removed orphaned frame: {}", "−".red(), slug);
        }
    }

    // ── Step 6: Write manifest ──────────────────────────────────────────
    // For filtered syncs, merge new entries with existing manifest
    let final_entries = if is_filtered {
        let mut merged = old_manifest
            .as_ref()
            .map(|m| m.frames.clone())
            .unwrap_or_default();

        // Update or insert entries for frames we just synced
        for entry in &manifest_entries {
            if let Some(existing) = merged.iter_mut().find(|e| e.id == entry.id) {
                *existing = entry.clone();
            } else {
                merged.push(entry.clone());
            }
        }
        merged
    } else {
        manifest_entries
    };

    let manifest = FigmaManifest {
        file_key: file_key.clone(),
        file_name: file.name.clone(),
        last_modified: file.last_modified,
        version: file.version,
        synced_at: chrono::Utc::now().to_rfc3339(),
        frames: final_entries,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, &manifest_json)?;

    // ── Summary ─────────────────────────────────────────────────────────
    let synced_count = frames.len() as u32 - skipped;
    println!(
        "\n{} Synced {} frames to .treble/figma/{}",
        "Done!".green().bold(),
        synced_count,
        if skipped > 0 {
            format!(" ({skipped} unchanged, skipped)")
        } else {
            String::new()
        }
    );
    if !orphaned.is_empty() {
        println!(
            "  {} {} orphaned frame(s) removed",
            "−".red(),
            orphaned.len()
        );
    }
    println!("\nChanges visible via: {}", "git diff .treble/".bold());

    Ok(())
}

// ── Interactive tree browser ─────────────────────────────────────────────

use crate::figma::types::CanvasNode;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{self, ClearType},
};
use std::io::Write;

struct PageNode {
    name: String,
    expanded: bool,
    frames: Vec<FrameNode>,
}

struct FrameNode {
    name: String,
    short_id: String,
    global_index: usize,
    selected: bool,
}

#[derive(Clone)]
enum VisibleRow {
    Page(usize),
    Frame(usize, usize),
}

fn build_visible(pages: &[PageNode]) -> Vec<VisibleRow> {
    let mut rows = Vec::new();
    for (pi, page) in pages.iter().enumerate() {
        rows.push(VisibleRow::Page(pi));
        if page.expanded {
            for (fi, _) in page.frames.iter().enumerate() {
                rows.push(VisibleRow::Frame(pi, fi));
            }
        }
    }
    rows
}

fn selected_count(page: &PageNode) -> (usize, usize) {
    let total = page.frames.len();
    let selected = page.frames.iter().filter(|f| f.selected).count();
    (selected, total)
}

fn interactive_select(
    canvas_pages: &[CanvasNode],
    all_frames: &[FrameInfo],
) -> Result<Vec<usize>> {
    // Build tree model
    let mut pages: Vec<PageNode> = canvas_pages
        .iter()
        .map(|cp| {
            let frames: Vec<FrameNode> = all_frames
                .iter()
                .enumerate()
                .filter(|(_, f)| f.page_name == cp.name)
                .map(|(gi, f)| FrameNode {
                    name: f.name.clone(),
                    short_id: f.short_id.clone(),
                    global_index: gi,
                    selected: false,
                })
                .collect();
            PageNode {
                name: cp.name.clone(),
                expanded: false,
                frames,
            }
        })
        .collect();

    let mut cursor_pos: usize = 0;

    // Enter raw mode
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();

    // Hide cursor
    execute!(stdout, cursor::Hide)?;

    let result = (|| -> Result<Vec<usize>> {
        loop {
            let visible = build_visible(&pages);
            let total_selected: usize = pages.iter().map(|p| p.frames.iter().filter(|f| f.selected).count()).sum();
            let total_frames: usize = pages.iter().map(|p| p.frames.len()).sum();

            // Clamp cursor
            if !visible.is_empty() && cursor_pos >= visible.len() {
                cursor_pos = visible.len() - 1;
            }

            // Get terminal height for scrolling
            let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
            let max_rows = (term_height as usize).saturating_sub(7); // header(3) + footer(4)
            let _width = term_width as usize;

            // Scroll window — keep cursor centered when possible
            let scroll_offset = if visible.len() <= max_rows {
                0
            } else if cursor_pos < max_rows / 2 {
                0
            } else if cursor_pos + max_rows / 2 >= visible.len() {
                visible.len().saturating_sub(max_rows)
            } else {
                cursor_pos - max_rows / 2
            };

            // Render
            let mut lines: Vec<String> = Vec::new();

            lines.push(String::new());
            lines.push(format!("  {}  {}",
                "Select frames to sync".bold(),
                format!("{total_selected}/{total_frames} selected").dimmed()));
            lines.push(String::new());

            // Figure out which page is last (for └─ vs ├─)
            let last_page_idx = pages.len().saturating_sub(1);

            for (i, row) in visible.iter().enumerate().skip(scroll_offset).take(max_rows) {
                let is_cursor = i == cursor_pos;

                match row {
                    VisibleRow::Page(pi) => {
                        let page = &pages[*pi];
                        let (sel, total) = selected_count(page);
                        let is_last_page = *pi == last_page_idx;
                        let branch = if is_last_page { "└─" } else { "├─" };

                        // [+]/[-] or green ◼ when all selected
                        let marker = if sel == total && total > 0 {
                            format!("[{}]", "x".green())
                        } else if page.expanded {
                            "[ ]".to_string()
                        } else {
                            "[ ]".to_string()
                        };

                        let name = clean_display_name(&page.name, 45);
                        let count_str = format!("({total})").dimmed().to_string();

                        if is_cursor {
                            lines.push(format!("  {} {} {} {}",
                                branch.dimmed(), marker, name.cyan().bold(), count_str));
                        } else {
                            lines.push(format!("  {} {} {} {}",
                                branch.dimmed(), marker.dimmed(), name, count_str));
                        }
                    }
                    VisibleRow::Frame(pi, fi) => {
                        let page = &pages[*pi];
                        let frame = &page.frames[*fi];
                        let is_last_page = *pi == last_page_idx;
                        let is_last_frame = *fi == page.frames.len() - 1;

                        let trunk = if is_last_page { "   " } else { "│  " };
                        let branch = if is_last_frame { "└─" } else { "├─" };

                        let name = clean_display_name(&frame.name, 40);
                        let id = &frame.short_id;
                        let check = if frame.selected {
                            format!("[{}]", "x".green())
                        } else {
                            "[ ]".to_string()
                        };

                        if is_cursor {
                            lines.push(format!("  {} {} {} {}  {}",
                                trunk.dimmed(), branch.dimmed(), check, name.cyan().bold(), id.dimmed().italic()));
                        } else if frame.selected {
                            lines.push(format!("  {} {} {} {}  {}",
                                trunk.dimmed(), branch.dimmed(), check, name.green(), id.dimmed().italic()));
                        } else {
                            lines.push(format!("  {} {} {} {}  {}",
                                trunk.dimmed(), branch.dimmed(), check, name.dimmed(), id.dimmed().italic()));
                        }
                    }
                }
            }

            // Footer
            lines.push(String::new());
            lines.push(format!("  {} navigate   {} select   {} sync   {} quit",
                "↑↓".dimmed(), "space".dimmed(), "enter".dimmed(), "q".dimmed()));
            lines.push(format!("  {} expand     {} all",
                "→←".dimmed(), "a".dimmed()));

            // Write
            execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))?;
            for line in &lines {
                write!(stdout, "{}\r\n", line)?;
            }
            stdout.flush()?;

            // Read key
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        // Signal cancellation with None
                        return Ok(Vec::new());
                    }
                    KeyCode::Up => {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                        }
                    }
                    KeyCode::Down => {
                        let visible = build_visible(&pages);
                        if cursor_pos + 1 < visible.len() {
                            cursor_pos += 1;
                        }
                    }
                    KeyCode::Right => {
                        let visible = build_visible(&pages);
                        if let Some(VisibleRow::Page(pi)) = visible.get(cursor_pos) {
                            pages[*pi].expanded = true;
                        }
                    }
                    KeyCode::Left => {
                        let visible = build_visible(&pages);
                        match visible.get(cursor_pos) {
                            Some(VisibleRow::Page(pi)) => {
                                pages[*pi].expanded = false;
                            }
                            Some(VisibleRow::Frame(pi, _)) => {
                                // Collapse parent and move cursor to it
                                pages[*pi].expanded = false;
                                // Find parent page row
                                let new_visible = build_visible(&pages);
                                cursor_pos = new_visible
                                    .iter()
                                    .position(|r| matches!(r, VisibleRow::Page(p) if *p == *pi))
                                    .unwrap_or(0);
                            }
                            None => {}
                        }
                    }
                    KeyCode::Char(' ') => {
                        let visible = build_visible(&pages);
                        match visible.get(cursor_pos) {
                            Some(VisibleRow::Page(pi)) => {
                                // Toggle all frames in page
                                let all_selected = pages[*pi]
                                    .frames
                                    .iter()
                                    .all(|f| f.selected);
                                let new_val = !all_selected;
                                for frame in &mut pages[*pi].frames {
                                    frame.selected = new_val;
                                }
                            }
                            Some(VisibleRow::Frame(pi, fi)) => {
                                pages[*pi].frames[*fi].selected =
                                    !pages[*pi].frames[*fi].selected;
                            }
                            None => {}
                        }
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    KeyCode::Char('a') => {
                        // Toggle all
                        let any_unselected = pages
                            .iter()
                            .any(|p| p.frames.iter().any(|f| !f.selected));
                        for page in &mut pages {
                            for frame in &mut page.frames {
                                frame.selected = any_unselected;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Collect selected
        let selected: Vec<usize> = pages
            .iter()
            .flat_map(|p| p.frames.iter())
            .filter(|f| f.selected)
            .map(|f| f.global_index)
            .collect();

        Ok(selected)
    })();

    // Restore terminal — always, even on error
    let _ = execute!(stdout, cursor::Show);
    let _ = terminal::disable_raw_mode();
    let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(ClearType::All));

    result
}

// ── Helpers ─────────────────────────────────────────────────────────────

struct SectionInfo {
    id: String,
    name: String,
}

/// Clean a name for display: strip junk chars (↳), collapse whitespace, cap length.
fn clean_display_name(name: &str, max: usize) -> String {
    let cleaned = name.replace('↳', "").replace('→', "");
    truncate_display(&cleaned, max)
}

/// Truncate a name for display: collapse whitespace to single spaces, cap length, add ellipsis.
fn truncate_display(name: &str, max: usize) -> String {
    let clean: String = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= max {
        clean
    } else {
        let truncated: String = clean.chars().take(max - 1).collect();
        format!("{truncated}…")
    }
}

fn find_sections(nodes: &[FlatNode], frame_width: Option<f64>) -> Vec<SectionInfo> {
    let min_width = frame_width.unwrap_or(1000.0) * 0.5;

    nodes
        .iter()
        .filter(|n| {
            n.depth == 1
                && n.node_type == "FRAME"
                && n.width.unwrap_or(0.0) > min_width
                && n.height.unwrap_or(0.0) > 50.0
        })
        .map(|n| SectionInfo {
            id: n.id.clone(),
            name: n.name.clone(),
        })
        .collect()
}
