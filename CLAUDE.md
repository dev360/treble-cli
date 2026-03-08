# treble

Figma to production code. Syncs your design to disk, analyzes it with AI, builds every component, and visually verifies the result — all from your terminal.

## CLI Commands

| Command | What it does |
|---------|-------------|
| `treble login` | Store Figma token (PAT or `--pat` flag) |
| `treble status` | Check auth + project state (`--json` for agents) |
| `treble init --figma <url>` | Scaffold `.treble/` in current project |
| `treble sync` | Pull Figma file → `.treble/figma/` (deterministic, git-friendly) |
| `treble tree "Frame"` | Print layer tree (offline, reads disk) |
| `treble show "Node" --frame "Frame"` | Render a Figma node screenshot (calls API) |
| `treble extract` | Extract image assets from synced frames |

## Plugin Commands (the brain)

| Command | What it does |
|---------|-------------|
| `/treble:sync` | Preflight checks, smart frame selection, sync Figma to disk |
| `/treble:plan` | Analyze Figma data → component inventory, design tokens, build order |
| `/treble:dev` | Classify design → pick stack → scaffold → build loop with visual review |
| `/treble:cms` | Wire up CMS editability (Sanity, Prismic, or WordPress) |
| `/treble:compare` | Visual comparison between Figma reference and implementation |

The CLI is the hands (Figma data access). The plugin commands are the brain (analysis + build orchestration).

## Architecture

```
src/
├── main.rs           # clap CLI, 5 subcommands
├── config.rs         # Global (~/.treble/) + project (.treble/) config
├── commands/
│   ├── login.rs      # Figma token storage (PAT mode)
│   ├── status.rs     # Auth + project state checker (--json for agents)
│   ├── init.rs       # Project scaffolding
│   ├── sync.rs       # Figma → disk sync (deterministic, orphan cleanup)
│   ├── tree.rs       # Layer tree printer (colored, with visual props)
│   ├── show.rs       # On-demand node rendering via Figma images API
│   └── extract.rs    # Image asset extraction
└── figma/
    ├── client.rs     # Figma REST API (files, nodes, images)
    └── types.rs      # API types + FlatNode + FigmaManifest

.claude-plugin/
├── marketplace.json      # Plugin registry
├── CLAUDE.md             # Plugin context (injected into Claude's awareness)
├── hooks.json            # SessionStart check
├── commands/
│   ├── sync.md           # Smart Figma sync with preflight + frame selection
│   ├── plan.md           # Analysis — design tokens, component inventory, build order
│   ├── dev.md            # Build router — classify, pick stack, scaffold, hand off
│   ├── cms.md            # CMS wiring — compatibility-gated options
│   ├── compare.md        # Visual comparison prompt
│   ├── tree.md           # Layer exploration
│   └── show.md           # Node rendering
└── skills/
    ├── dev-shadcn.md     # Build loop for React + shadcn/ui targets
    └── dev-basecoat-wp.md # Build loop for WordPress + Basecoat targets
```

## Dev

```bash
mise run build        # cargo build --release
mise run install      # build + install to ~/.cargo/bin
mise run test         # cargo test
mise run lint         # clippy + fmt check
```

**IMPORTANT:** Before pushing, ALWAYS run these checks:
```bash
cargo fmt          # Fix formatting (CI enforces cargo fmt --check)
cargo clippy       # Catch lint warnings
cargo test         # Run tests
cargo build        # Verify it compiles
```
CI will reject the push if `cargo fmt --check` fails. Fix formatting BEFORE committing.

**IMPORTANT:** After ANY code change, always build and install immediately:
```bash
mise run install
```
This ensures the user's `treble` binary in PATH is always up to date.

## Publishing

- Pushes to `main` trigger semantic-release via GitHub Actions
- Commit prefixes: `fix:` → patch, `feat:` → minor, `BREAKING CHANGE` → major
- `chore:`, `docs:`, `style:`, `test:`, `ci:` do NOT trigger a release
- CI cross-compiles for darwin-arm64/x64 + linux-x64/arm64, publishes to npm as `@treble-app/cli`

## Auth

The CLI supports two token types for Figma API:
- **OAuth** (`treble login` device flow): uses `Authorization: Bearer` header
- **PAT** (`treble login --pat`): uses `X-Figma-Token` header

`GlobalConfig::figma_client()` auto-detects which to use based on `session_token` presence.
`FigmaClient::new()` = PAT, `FigmaClient::new_oauth()` = OAuth Bearer.
