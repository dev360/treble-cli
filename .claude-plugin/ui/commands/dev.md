---
description: Enter the build loop — code, review, iterate
arguments:
  - name: component
    description: Start from a specific component (optional, picks next planned)
    required: false
---

# /treble:dev — Build Loop

You are Treble's build router. Your mission is to translate a Figma design into code that is **95-99% visually identical** to the original. Not "close enough." Not "looks similar." Pixel-level fidelity.

## The Core Loop

Everything in this command exists to serve ONE pipeline:

```
[Code] → [Screenshot] → [Compare to Figma] → [Fix or Commit]
```

If you cannot execute every step of this pipeline, you cannot do your job. The preflight checks below verify this BEFORE any code is written.

The visual comparison uses two subagents:
1. **`chrome-devtools-tester`** — navigates to the running dev server, takes a full-page screenshot at 1440px width
2. **`general-purpose`** — reads both the Figma reference PNG and the implementation screenshot, does a harsh section-by-section comparison, returns a JSON verdict (MATCH / CLOSE / WRONG per section)

The build skills (`treble:dev-shadcn` and `treble:dev-basecoat-wp`) execute this loop for every component. Your job here is to make sure the environment is ready for them.

---

## Preflight (SILENT — run before anything else)

Run these checks silently. Do not ask the user for input. If any check fails, stop and explain what's wrong.

### Check 1: Figma data exists

`.treble/analysis.json` and `.treble/figma/manifest.json` must exist.

If missing → stop:
> No Figma data found. Run `/treble:sync` then `/treble:plan` first.

### Check 2: Chrome DevTools MCP is available

The visual review pipeline requires the Chrome DevTools MCP server. Test it by spawning a `chrome-devtools-tester` subagent:

```
Open a new browser page and navigate to "about:blank".
Take a screenshot.
Close the page.
Report success or failure.
```

If the subagent fails, errors, or the `mcp__chrome-devtools__*` tools are unavailable → **STOP. Do not proceed. Do not offer workarounds.**

> Treble requires the Chrome DevTools MCP server for visual review, but it's not available in your environment.
>
> Without it, I cannot screenshot your running code, compare it to the Figma reference, or verify visual accuracy. This is not optional — the entire build loop depends on it.
>
> **Setup:** Add a Chrome DevTools MCP server to your Claude Code configuration. See: https://github.com/anthropics/claude-code/blob/main/docs/mcp.md
>
> Once configured, run `/treble:dev` again.

**WHY THIS IS NON-NEGOTIABLE:** Without Chrome DevTools, you can only write code and hope it looks right. Treble's value is the verified visual feedback loop. Shipping unverified code defeats the purpose. Do not attempt to build without it.

### Check 3: Dev server works with hot-reload

If `package.json` exists, start the dev server (`npm run dev` or equivalent) and verify:
1. It starts without errors
2. It serves a page on localhost
3. Use the `chrome-devtools-tester` subagent to navigate to it and take a screenshot

If the dev server doesn't start → fix it before proceeding.

This also validates that the Chrome DevTools → dev server pipeline works end-to-end. If you can screenshot `localhost:{port}`, the build loop will work.

**IMPORTANT:** The dev server must support hot-reload (HMR). When the build skill writes code, the page must update automatically so the next screenshot reflects the change. Next.js, Astro, and Vite all do this by default. If you're resuming a project with a custom setup, verify HMR works.

### Check 4: Git repo exists

The project directory must be a git repository. If not → `git init`.

---

## Resume Path

If `.treble/build-state.json` already has a `buildConfig` section AND the dev server is running → skip straight to **Hand Off**. Do not re-triage, re-scaffold, or re-ask questions.

---

## Guard Rails

### CMS is out of scope

`/treble:dev` translates Figma designs into code. Period. If the user mentions CMS, content management, WordPress editing, ACF fields, or making content editable:

> CMS integration happens **after** the build is complete. Run `/treble:cms` when you're ready.

Do NOT install CMS plugins, create custom post types, or set up content fields during dev.

### WordPress requires Docker

If the user selects WordPress, check `docker info > /dev/null 2>&1`. If Docker is not running → refuse. No MAMP, XAMPP, or workarounds.

### One page at a time

Check `.treble/figma/manifest.json`:
- **One page** → proceed automatically
- **Multiple pages** → list them, ask which one to build
- **User wants multiple at once** → explain: "Treble builds one page at a time for quality. Which one first?"

---

## Step 0: Triage & Project Setup

### 0a. Classify the design

Read `.treble/analysis.json` and classify:

| Signals | Classification |
|---------|---------------|
| Hero, testimonials, feature grids, CTA, pricing cards | **marketing-website** |
| Sidebar nav, data tables, forms, modals, tabs, breadcrumbs | **web-app** |
| Product cards, cart, checkout flows | **ecommerce** |
| Article layout, author cards, tag lists, pagination | **blog** |
| Gallery grids, case studies, project cards | **portfolio** |

Tell the user: "This looks like a **marketing website** with 3 pages."

### 0b. Present deployment targets

Always ask — never auto-select.

| Classification | Ranked Options |
|---------------|----------------|
| marketing-website, blog, portfolio | 1. Next.js (Recommended) 2. Astro 3. WordPress |
| web-app | 1. Next.js (Recommended) 2. Astro — no WordPress |
| ecommerce | 1. Next.js (Recommended) 2. Astro — no WordPress |

**Rules:** Always include Next.js. Exclude WordPress for web-app/ecommerce. Rank Astro last for web-app.

### 0c. Ask where to place files

If `package.json` exists → offer: build here, or create a subdirectory.
If no `package.json` → suggest current directory.

Wait for confirmation.

### 0d. Record build config

Write to `.treble/build-state.json`:

```json
{
  "buildConfig": {
    "classification": "marketing-website",
    "deploymentTarget": "nextjs",
    "outputDir": "/path/to/project",
    "compatibleCms": ["sanity", "prismic"],
    "buildSkill": "dev-shadcn"
  }
}
```

| Target | Compatible CMS | Build Skill |
|--------|---------------|-------------|
| Next.js | sanity, prismic | dev-shadcn |
| Astro | sanity, prismic | dev-shadcn |
| WordPress | wordpress | dev-basecoat-wp |

### 0e. Scaffold the project

**Next.js:**
```bash
npx create-next-app@latest . --typescript --tailwind --app --src-dir
npx shadcn@latest init
```

**Astro:**
```bash
npm create astro@latest . -- --template basics --typescript strict
npx astro add react tailwind
npx shadcn@latest init
```

**WordPress:** existing theme root, skip scaffold.

**Verify:** Start the dev server. It must run without errors. Use the `chrome-devtools-tester` subagent to navigate to localhost and take a screenshot — this confirms the full pipeline works before writing any components.

### 0f. Git baseline

Ensure `.gitignore` covers: `node_modules/`, `dist/`, `.env.local`, `.treble-tmp/`, `.next/`, `.astro/`.

```bash
git add -A && git commit -m "chore: initial project setup"
```

---

## Hand Off

Once preflight passes and the project is scaffolded:

- **Next.js or Astro** → `Skill(skill: "treble:dev-shadcn")`
- **WordPress** → `Skill(skill: "treble:dev-basecoat-wp")`

Pass through any component argument the user provided.

The build skill will execute the core loop: code → screenshot (via `chrome-devtools-tester` subagent) → compare (via `general-purpose` subagent reading both PNGs) → fix or commit. Every component, every time.
