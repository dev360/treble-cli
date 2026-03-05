# Treble Plugin v0.3.0

Figma-to-code build assistant. Syncs Figma designs to disk, Claude analyzes them and builds components.

## CRITICAL: Data Source Rules

**ONLY use the `treble` CLI and local `.treble/` files for all Figma data.**

- Do NOT call the Figma REST API directly
- Do NOT use any Figma MCP server or plugin
- Do NOT use `curl`/`fetch` against api.figma.com
- All Figma data has been synced to disk by `treble sync`. Work with the local files.

The only exception is `treble show`, which calls the Figma images API to render a specific node — but you invoke it via the CLI, not directly.

## Commands

| Command | What it does |
|---------|-------------|
| `/treble:plan` | Analyze Figma frames → write analysis.json + build-state.json |
| `/treble:dev` | Build loop: code → visual review → architectural review → iterate |
| `/treble:tree` | Show the Figma layer outline for a frame |
| `/treble:show` | Render a specific Figma node as a screenshot |
| `/treble:compare` | Compare implementation against Figma reference |

## CLI Tool Reference

```bash
treble login --pat                       # Store Figma token
treble init --figma "URL_OR_KEY"         # Scaffold .treble/ in project
treble sync                              # Pull all Figma frames to disk
treble sync -i                           # Interactive frame picker
treble sync --page "Homepage"            # Sync frames from one page
treble sync --frame "Contact"            # Sync one frame by name
treble sync --force                      # Re-sync even if already on disk
treble tree "Contact" --depth 2          # Layer outline (top 2 levels)
treble tree "Contact" --verbose          # With fills, fonts, layout, radius
treble tree "Contact" --root "55:1234"   # Subtree rooted at a specific node
treble tree "Contact" --root "55:1234" --json   # Machine-readable subtree
treble show "NavBar" --frame "Contact"   # Render a specific node as PNG
treble show "55:1234"                    # Render by node ID
```

## Disk Structure

```
.treble/
├── config.toml              # Figma file key + flavor
├── analysis.json            # YOUR analysis output (components, design system, build order)
├── build-state.json         # YOUR build progress (status per component)
└── figma/                   # Synced Figma data (from `treble sync`)
    ├── manifest.json        # Frame inventory (names, IDs, slugs, node counts)
    └── {frame-slug}/        # One dir per frame
        ├── reference.png    # Full frame screenshot
        ├── nodes.json       # Complete node tree with all visual properties
        ├── sections/        # Section-level screenshots (depth-1 frames)
        └── snapshots/       # On-demand screenshots (from `treble show`)
```

## Workflow

1. `treble sync` — Pull Figma data to disk
2. `/treble:plan` — Analyze the screenshots + node trees, write analysis.json
3. `/treble:dev` — Implement components in build order, reviewing each one

## How to Explore a Design (Slicing)

Start broad, then drill in. Never read the full nodes.json for large frames.

1. **See what's synced:** `cat .treble/figma/manifest.json`
2. **See the full page:** `Read .treble/figma/{slug}/reference.png`
3. **List all sections:** `treble tree "Homepage" --depth 1` → shows IDs
4. **See one section visually:** `treble show "55:1234" --frame "Homepage"` → renders that node as PNG
5. **Get section structure:** `treble tree "Homepage" --root "55:1234" --verbose`
6. **Get machine-readable data:** `treble tree "Homepage" --root "55:1234" --json`
7. **See section screenshots:** `Read .treble/figma/{slug}/sections/*.png`

## Two Files, Two Concerns

**analysis.json** — What to build. Written by `/treble:plan`. Components, design system, build order, Figma node references. Immutable after planning.

**build-state.json** — Build progress. Written by `/treble:dev`. Status, file paths, review results per component. Updated as you build.

## Version Control

All `.treble/` changes should be committed to git:
- `git diff .treble/analysis.json` — see what changed in the analysis
- `git log .treble/build-state.json` — track build progress over time
