---
description: Show the Figma layer tree for a frame
arguments:
  - name: frame
    description: Frame name (e.g. "Contact", "Home")
    required: true
---

# /treble:tree — Figma Layer Outline

Show the layer hierarchy for a synced Figma frame. Use this to understand structure before planning or building.

## Usage

```bash
# Show full tree
treble tree "Contact"

# Limit depth (top 2 levels only)
treble tree "Contact" --depth 2

# Show with visual properties (fills, fonts, layout, radius)
treble tree "Contact" --verbose

# Show only a specific subtree (by node ID or name)
treble tree "Contact" --root "55:1234" --verbose

# Machine-readable JSON output (compact, with hex colors and font info)
treble tree "Contact" --root "55:1234" --json

# Combine: subtree at depth 2 in JSON
treble tree "Contact" --root "55:1234" --depth 2 --json
```

## What it shows

For each layer:
- **Type badge**: FRAME, TEXT, RECT, VEC, COMP (component), INST (instance), GRP (group)
- **Name**: the Figma layer name
- **Size**: width x height in pixels
- **Node ID**: the Figma node ID (use this for `--root` and `treble show`)
- **Auto-layout**: direction (HORIZONTAL/VERTICAL) if present
- **Text content**: actual text strings for TEXT nodes
- **Child count**: how many children the node has

With `--verbose`: fill colors, font family/size/weight, layout padding/gap, corner radius.

With `--json`: compact JSON array with id, name, type, depth, width, height, x, y, fills (hex), font info, radius. No ANSI colors, parseable by tools.

## Slicing with --root

The `--root` flag is the key tool for analyzing large frames section by section:

1. **List sections:** `treble tree "Homepage" --depth 1` → see all depth-1 children with their IDs
2. **Drill in:** `treble tree "Homepage" --root "366:10537" --verbose` → see just that section's subtree
3. **Get data:** `treble tree "Homepage" --root "366:10537" --json` → machine-readable for analysis

The `--root` accepts either a node ID (`"55:1234"`) or a name (`"NavBar"`). When using names, it does a fuzzy case-insensitive match.

## When to use

- **Before `/treble:plan`**: see what layers exist, get node IDs for slicing
- **During `/treble:plan`**: drill into sections one at a time with `--root`
- **During `/treble:dev`**: understand the structure of a section you're implementing
- **Debugging**: check if synced data matches what you expect from Figma
