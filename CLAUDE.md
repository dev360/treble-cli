# treble-cli

Rust CLI that syncs Figma designs to disk. The Claude plugin provides the intelligence — analysis prompts, build loop, review cycle.

## CLI Commands

| Command | What it does |
|---------|-------------|
| `treble login` | Store Figma token (PAT, device flow, or `--figma-token` flag) |
| `treble init --figma <url>` | Scaffold `.treble/` in current project |
| `treble sync` | Pull Figma file → `.treble/figma/` (deterministic, git-friendly) |
| `treble tree "Frame"` | Print layer tree (offline, reads disk) |
| `treble show "Node" --frame "Frame"` | Render a Figma node screenshot (calls API) |

## Plugin Commands (the brain)

| Command | What it does |
|---------|-------------|
| `/treble:plan` | Claude analyzes Figma data → writes `analysis.json` + `build-state.json` |
| `/treble:dev` | Claude enters build loop: code → visual review → architectural review → iterate |
| `/treble:compare` | Claude compares implementation vs Figma reference |

The CLI is just the hands (Figma data access). The plugin commands are the brain (analysis + build orchestration).

## Architecture

```
src/
├── main.rs           # clap CLI, 5 subcommands
├── config.rs         # Global (~/.treble-cli/) + project (.treble/) config
├── commands/
│   ├── login.rs      # Figma token storage (3 modes: device, PAT, flag)
│   ├── init.rs       # Project scaffolding
│   ├── sync.rs       # Figma → disk sync (deterministic, orphan cleanup)
│   ├── tree.rs       # Layer tree printer (colored, with visual props)
│   └── show.rs       # On-demand node rendering via Figma images API
└── figma/
    ├── client.rs     # Figma REST API (files, nodes, images)
    └── types.rs      # API types + FlatNode + FigmaManifest

.claude-plugin/
├── marketplace.json      # Plugin registry
├── CLAUDE.md             # Plugin context (injected into Claude's awareness)
├── hooks.json            # SessionStart check
└── commands/
    ├── plan.md           # Analysis system prompt — full design analysis workflow
    ├── dev.md            # Build loop — code → visual review → arch review → iterate
    ├── compare.md        # Visual comparison prompt
    ├── tree.md           # Layer exploration
    └── show.md           # Node rendering
```

## Dev

```bash
mise run build        # cargo build --release
mise run install      # build + install to ~/.cargo/bin
mise run test         # cargo test
mise run lint         # clippy + fmt check
```

**IMPORTANT:** After ANY code change, always build and install immediately:
```bash
mise run install
```
This ensures the user's `treble` binary in PATH is always up to date.
