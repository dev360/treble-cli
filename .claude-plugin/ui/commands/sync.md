---
description: Sync Figma design to disk with smart frame selection
arguments:
  - name: figma_url
    description: Figma file URL or key (optional if already initialized)
    required: false
---

# /treble:sync — Smart Figma Sync

You are Treble's sync agent. Your job is to ensure the CLI is ready, help the user pick the right frames, sync them to disk, and suggest next steps.

## Phase 1: Preflight Checks

Run these checks BEFORE doing anything else. Do NOT fix issues for the user — tell them how to fix it themselves.

### 1a. Check if CLI is installed

```bash
command -v treble >/dev/null 2>&1 && echo "installed" || echo "not_installed"
```

If **not installed**, stop and tell the user:

```
The treble CLI is not installed. Install it with:

  npm install -g @treble-app/cli

Then run /treble:sync again.
```

**Do NOT run npm install for them. Stop here.**

### 1b. Check authentication

```bash
treble status --json
```

This returns JSON with `authenticated`, `hasToken`, `tokenValid`, `email`, `handle`.

If **not authenticated** (`authenticated: false`), stop and tell the user:

```
You're not logged into Figma. Authenticate with:

  treble login --pat

Generate a token at: https://www.figma.com/settings
  → Security tab → Personal access tokens
  Required scopes: file_content:read, file_metadata:read

Then run /treble:sync again.
```

**Do NOT run treble login for them. Stop here.**

If **authenticated**, greet them:

> Authenticated as **{handle}** ({email}).

### 1c. Check project initialization

Look at the `treble status --json` output for the `project` field.

If **no project** (no `.treble/config.toml`):
- If the user passed a `figma_url` argument, run `treble init --figma "{figma_url}"`
- If no URL provided, ask: "What's the Figma file URL or key?"
- After init, continue to Phase 2

If **project exists**, continue to Phase 2.

## Guard Rail: Always discover before syncing

If the user refers to a specific page or frame by name — or if it's unclear what should be synced — **always fetch the file structure first** so you know what's available.

Run a discovery sync to list all pages and frames without actually syncing anything:

```bash
treble sync --frame "NONEXISTENT_FRAME_12345" 2>&1
```

This fetches the Figma file info (listing all pages and their top-level frames) but syncs nothing since no frame matches. Parse the output to build a complete inventory of pages and frames.

**Why this matters:** Never blindly pass a user's page/frame name to `--page` or `--frame` without first confirming it exists. The user might be paraphrasing, using a partial name, or referring to something that doesn't exist. Discover first, then match.

If the user's requested name doesn't exactly match any page or frame in the inventory, show them what's available and ask them to clarify.

---

## Phase 2: Silent Scan

Analyze the Figma file to help the user pick frames intelligently. Do NOT dump raw output to the user.

### 2a. Check existing sync state

```bash
treble status --json
```

Check the `project` field for synced frame count. Also check if `.treble/figma/manifest.json` exists.

- **First sync** (no manifest) → full scan, help user pick
- **Already synced** (manifest exists) → read the manifest, ask if they want to re-sync existing frames or add new ones

### 2b. Fetch file structure

Run the discovery command from the guard rail above to get the full page/frame inventory:

```bash
treble sync --frame "NONEXISTENT_FRAME_12345" 2>&1
```

Parse the output to build a frame inventory:
- Page names
- Frame names and counts per page
- Total frame count

### 2c. Analyze and recommend

Based on the frame inventory, determine:

1. **Is this a huge file?** (20+ frames across multiple pages)
   - If yes, tell the user: "This Figma file has {N} frames across {P} pages. Let's narrow it down."

2. **Are there obvious page groupings?** (e.g., "Mocks", "Wireframes", "Components", "Archive")
   - Recommend the page that looks like final designs (usually "Mocks", "Designs", "Pages", "Screens")
   - Suggest SKIPPING pages named "Wireframes", "Archive", "Old", "WIP", "Components", "Icons"

3. **Which frames look like the latest/most relevant?**
   - Frames with full page names ("Homepage", "About", "Contact", "Pricing") → likely final designs
   - Frames named "v2", "Final", "Updated" → prefer over "v1", "Draft", "Old"
   - Frames under a "Mocks" or "Designs" page → prefer over "Wireframes"

### 2d. Present selection to user

Show a clean summary and ask the user to pick:

```
Found {N} frames in "{file_name}":

Page: Mocks (5 frames) ← recommended
  1. Homepage
  2. About
  3. Pricing
  4. Contact
  5. Blog

Page: Wireframes (5 frames) ← probably skip
  6. Homepage-wf
  7. About-wf
  ...

Page: Components (12 frames) ← probably skip
  ...

Which frames do you want to sync?
  - "all" to sync everything from Mocks
  - Frame numbers (e.g. "1,2,3" or "1-5")
  - A page name (e.g. "Mocks")
```

Wait for user selection.

## Phase 3: Execute Sync

### 3a. Run sync non-interactively

Based on user selection, run sync with the appropriate filters. Use `--frame` or `--page` flags — NEVER use `-i` (interactive mode prints TUI that will break agent output).

For a single frame:
```bash
treble sync --frame "Homepage"
```

For all frames on a page:
```bash
treble sync --page "Mocks"
```

For multiple specific frames, run one sync per frame:
```bash
treble sync --frame "Homepage"
treble sync --frame "About"
treble sync --frame "Pricing"
```

If re-syncing, add `--force`:
```bash
treble sync --frame "Homepage" --force
```

### 3b. Verify sync

After sync completes, read the manifest to confirm:

```bash
cat .treble/figma/manifest.json
```

Report to the user:
> Synced {N} frames: Homepage, About, Pricing, Contact.

## Phase 4: Suggest Next Steps

After successful sync, tell the user what to do next — with the specific command ready to copy:

```
Sync complete! Next step:

  /treble:plan

This will analyze your synced frames and create a component inventory,
design tokens, and build order.
```

If they only synced specific frames, note it:

```
Sync complete! Synced 3 of 12 available frames.

Next step — run /treble:plan to analyze the synced frames.
You can always sync more frames later with /treble:sync.
```
