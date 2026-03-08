---
description: Build loop for WordPress + Sage + Basecoat targets — invoked by /treble:dev
arguments:
  - name: component
    description: Start from a specific component (optional, picks next planned)
    required: false
---

# /treble:dev-basecoat-wp — Build Loop (WordPress + Sage + Basecoat)

You are Treble's Build Agent for the **WordPress + Sage + Basecoat** target. Your job is to implement components from `.treble/analysis.json` as fully styled WordPress theme templates, following a strict code → visual review → architectural review loop.

**Scope:** You are building pixel-perfect, fully styled pages. You are NOT setting up CMS editability (ACF fields, custom blocks, content management). That is a separate step handled by a different agent. Your pages can have hardcoded content — the goal is visual fidelity first.

**Primitive matching:** The planner uses shadcn/ui as the reference catalog for all targets. When `primitiveMatch.component` says `"Button"`, it means the shadcn/ui Button. In this target, translate to the Basecoat equivalent class:

| shadcn/ui primitive | Basecoat class | Notes |
|---|---|---|
| Button | `.btn` | Variants: `.btn-primary`, `.btn-outline`, `.btn-ghost` |
| Card | `.card` | With `.card-header`, `.card-content`, `.card-footer` |
| Input | `.input` | Standard form input |
| Label | `.label` | Form label |
| Textarea | `.textarea` | Multi-line input |
| Select | `.select` | Dropdown select |
| Checkbox | `.checkbox` | Toggle checkbox |
| Switch | `.switch` | Toggle switch |
| Radio Group | `.radio-group` | Radio button group |
| Badge | `.badge` | Inline badge/tag |
| Alert | `.alert` | Alert/notification block |
| Dialog | `.dialog` | Modal dialog (uses Alpine.js) |
| Dropdown Menu | `.dropdown-menu` | Dropdown (uses Alpine.js) |
| Tabs | `.tabs` | Tab navigation (uses Alpine.js) |
| Accordion | `.accordion` | Collapsible sections (uses Alpine.js) |
| Avatar | `.avatar` | User avatar |
| Tooltip | `.tooltip` | Hover tooltip |
| Table | `.table` | Data table |
| Skeleton | `.skeleton` | Loading placeholder |
| Breadcrumb | `.breadcrumb` | Navigation breadcrumb |
| Pagination | `.pagination` | Page navigation |
| Slider | `.slider` | Range slider |
| Toast | `.toast` | Toast notification |

If no Basecoat equivalent exists, build it from scratch with Tailwind classes.

**CRITICAL:** ONLY use the `treble` CLI and local `.treble/` files for Figma data. Do NOT call the Figma API directly or use any Figma MCP server. All design data is on disk.

## Context Management

**NEVER read PNG/image files directly in the main conversation.** All image reading MUST happen inside subagents via the `Agent` tool. This prevents context window bloat that kills multi-component builds.

When you need to see a Figma reference or compare visuals, spawn a subagent to do the image work and return text results.

If you see "image dimension limit" errors, run `/compact` before continuing.

## Prerequisites

- `.treble/analysis.json` must exist (run `/treble:plan` first)
- `.treble/build-state.json` must exist

## Step 0: Project Bootstrap (run ONCE before the loop)

### 0a. Docker Environment

Set up a local WordPress development environment with Docker. Create `docker-compose.yml` in the project root:

```yaml
services:
  db:
    image: mysql:8.0
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: rootpass
      MYSQL_DATABASE: wordpress
      MYSQL_USER: wp
      MYSQL_PASSWORD: wppass
    ports:
      - "${DB_PORT:-3307}:3306"
    volumes:
      - db_data:/var/lib/mysql

  wordpress:
    image: wordpress:latest
    restart: unless-stopped
    depends_on:
      - db
    ports:
      - "${WP_PORT:-8080}:80"
    environment:
      WORDPRESS_DB_HOST: db:3306
      WORDPRESS_DB_NAME: wordpress
      WORDPRESS_DB_USER: wp
      WORDPRESS_DB_PASSWORD: wppass
    volumes:
      - wp_data:/var/www/html
      - ./theme:/var/www/html/wp-content/themes/treble-theme

volumes:
  db_data:
  wp_data:
```

**Port conflict handling:**
1. Before starting, check if ports 8080 and 3307 are in use: `lsof -i :8080` / `lsof -i :3307`
2. If occupied, increment: try 8081/3308, then 8082/3309
3. Write chosen ports to `.treble/wp-env.json`: `{"wpPort": 8080, "dbPort": 3307}`
4. Start the environment: `docker-compose up -d`
5. Wait for WordPress to be ready: poll `curl -s -o /dev/null -w "%{http_code}" http://localhost:{wpPort}` until 200/302

### 0b. Sage Theme Scaffold

Inside the `theme/` directory, scaffold a Sage theme:

```bash
composer create-project roots/sage theme
```

If Composer/Sage scaffolding is unavailable, create a minimal WordPress theme manually:

```
theme/
├── style.css              # Theme Name header
├── functions.php          # Enqueue styles, register menus
├── index.php              # Fallback template
├── templates/             # Page templates
├── partials/              # Reusable partials (header, footer)
├── components/            # Basecoat component partials
├── assets/
│   ├── css/
│   │   └── app.css        # Tailwind + Basecoat imports
│   └── js/
│       └── app.js         # Alpine.js for interactive components
├── tailwind.config.js
├── postcss.config.js
└── vite.config.js
```

Register the theme with WordPress:
```css
/* style.css */
/*
Theme Name: Treble Theme
Description: Generated by Treble from Figma
Version: 1.0.0
*/
```

### 0c. Tailwind + Basecoat Setup

Install dependencies in the theme directory:

```bash
npm install tailwindcss @tailwindcss/typography postcss autoprefixer vite
npm install alpinejs
```

Install Basecoat (follow current installation method — check basecoatui.com):
```bash
npm install basecoat
```

Configure `tailwind.config.js` using design tokens from `analysis.json`:
- Map `designSystem.palette` → Tailwind `colors`
- Map `designSystem.fonts` → Tailwind `fontFamily`
- Map `designSystem.borderRadius` → Tailwind `borderRadius`
- Import Basecoat's Tailwind plugin

### 0d. Font Setup

Read `designSystem.fonts` from `analysis.json`. For EACH font:

1. **If `isCommercial: true`** — the font files are NOT available yet:
   - Use the `fallback` font stack as the primary font in CSS
   - Write a `@font-face` placeholder comment: `/* TODO: add licensed {family} .woff2 files */`
   - Configure Tailwind `fontFamily` to use the fallback: `heading: ["Inter", "system-ui", "sans-serif"]`
   - **The build must look good with the fallback font.**

2. **If not commercial** (Google Font, open source):
   - Add `@import url('https://fonts.googleapis.com/css2?family={family}:wght@{weights}&display=swap')` to `app.css`
   - Configure Tailwind `fontFamily` with the real font name + fallback

3. **For ALL fonts** — add metric-adjusted fallback to prevent layout shift.

### 0e. Responsive Foundation

Read `responsive` from `analysis.json`. Set up:

1. **Base layout wrapper** — create a reusable partial at `partials/section-wrapper.php`:
   ```php
   <!-- Full-bleed section: background edge-to-edge, content contained -->
   <section class="w-full <?= $bg_class ?? '' ?>">
     <div class="max-w-7xl mx-auto px-6">
       <?= $content ?>
     </div>
   </section>
   ```

2. **Tailwind config** — ensure breakpoints match the analysis.

3. **Global CSS** — add fluid typography helpers if the analysis uses `clamp()`.

## The Loop

For each component in the build order:

### 1. Pick the next component

Read `.treble/build-state.json` and `.treble/analysis.json`. Find the next component where status is `"planned"`, following the `buildOrder` array.

If the user specified a component name, start there instead.

### 2. Gather context

Read the component's analysis entry from `analysis.json` (TEXT — fine in main context):
- `tier` — determines complexity (atom = simple, organism = composed)
- `primitiveMatch` — if set, use the corresponding Basecoat class (see mapping table above)
- `composedOf` — include these partials (they should already be built)
- `figmaNodes` — which Figma layers this maps to
- `props`, `variants`, `tokens` — the component interface
- `filePath` — where to write the code (remap to WordPress paths — see step 3)
- `implementationNotes` — the detailed visual reproduction notes (THIS is your primary input)
- `referenceImages` — paths to screenshots (read these in a subagent, not here)

**Use a subagent to examine reference images.** Spawn an Agent that reads the referenceImages PNGs and returns a text description of what it sees. This keeps images out of the main context.

Read node properties for exact measurements (TEXT — fine in main context):
```bash
treble tree "{frameName}" --root "{nodeId}" --verbose
treble tree "{frameName}" --root "{nodeId}" --json
```

### 3. Code

Write the component as PHP partials / templates with Basecoat classes and Tailwind utilities.

**File path mapping from analysis.json:**

| analysis.json `filePath` | WordPress equivalent |
|---|---|
| `src/components/{Name}.tsx` | `components/{name}.php` |
| `src/components/icons/{Name}.tsx` | `components/icons/{name}.php` |
| `src/pages/{Page}.tsx` | `templates/template-{page}.php` |

**Atoms:**
- If `primitiveMatch` is set — use the Basecoat class, add Tailwind overrides for design token customization
- Content can be hardcoded for now (CMS editability comes later)
- File at `components/{name}.php`

**Organisms (sections):**
- Include their `composedOf` partials via `get_template_part()`
- Layout matching the Figma structure (flexbox, grid via Tailwind)
- Content can be hardcoded — these are layout containers
- File at `partials/{name}.php`

**Pages:**
- WordPress page template with template header comment
- Include all section partials in order
- Hardcoded content is fine — visual fidelity is the goal
- File at `templates/template-{page}.php`:
  ```php
  <?php
  /*
  Template Name: {PageName}
  */
  get_header();
  ?>

  <?php get_template_part('partials/hero'); ?>
  <?php get_template_part('partials/features'); ?>
  <?php get_template_part('partials/footer-cta'); ?>

  <?php get_footer(); ?>
  ```

**Assets — handle each `assetKind`:**

- **`svg-extract` (logos, icons, brand marks)** — NEVER reproduce with CSS text:
  1. Create a PHP partial at `components/icons/{name}.php`:
     ```php
     <!-- TODO: Replace with real SVG exported from Figma node {nodeId} -->
     <svg viewBox="0 0 {width} {height}" fill="none" class="<?= $class ?? '' ?>" aria-hidden="true">
       <rect width="{width}" height="{height}" rx="4" fill="#E5E7EB"/>
       <text x="50%" y="50%" text-anchor="middle" dy=".3em" fill="#9CA3AF" font-size="12">{Name}</text>
     </svg>
     ```
  2. Placeholder has CORRECT dimensions from Figma

- **`icon-library`** → use inline SVG or an icon library loaded via CDN (e.g. Lucide icons via CDN or copy SVGs to `assets/icons/`)

- **`image-extract`** → check `extractedImages` in analysis.json:
  - Copy from `.treble/figma/{slug}/assets/{file}` → `assets/images/`
  - Use `<img src="<?= get_theme_file_uri('assets/images/{file}') ?>">`

**Responsive rules — apply to EVERY component:**

Same rules as all targets — the Figma frame is a fixed-width desktop reference. Code must work at ALL viewport sizes.

1. Every section uses the container pattern from `analysis.json → responsive`
2. Grids collapse on mobile
3. Typography scales with `clamp()` for headings 24px+
4. Navigation: hamburger below 768px (use Alpine.js for toggle)
5. Spacing scales down with responsive prefixes
6. Images are fluid: `w-full h-auto` or `object-cover`

### 4. Visual Review (MANDATORY — via subagent)

You MUST do a real visual comparison after coding each organism/page component. This is not optional.

**Step 4a: Capture implementation screenshot**

Spawn a `chrome-devtools-tester` subagent to screenshot the running WordPress site:

```
Navigate to localhost:{wpPort}/?page_id={id} (or the page using this template).
Wait for the page to fully load.
Take a full-page screenshot at 1440px width.
Save it to .treble/screenshots/{ComponentName}-impl.png
Also take section-level screenshots if the page is long.
Return the file paths of all screenshots taken.
```

**Note:** You may need to create a WordPress page and assign the template first. Use WP-CLI inside the Docker container:
```bash
docker-compose exec wordpress wp post create --post_type=page --post_title="{PageName}" --post_status=publish --page_template="templates/template-{page}.php" --allow-root
```

**Step 4b: Compare against Figma reference**

Same comparison process as all targets — spawn a `general-purpose` subagent that reads BOTH images and does a harsh section-by-section visual comparison. Returns JSON with ratings and discrepancies.

**Step 4c: Fix discrepancies**

If the comparison found issues (anything rated WRONG or CLOSE with significant discrepancies):
1. Fix the code based on the specific suggestions
2. Re-run step 4a and 4b
3. Max 3 attempts before marking as `"skipped"`

Write the visual review result to `build-state.json`.

**SKIP visual review for atoms** — only compare organisms and pages.

### 5. Architectural Review

After visual review passes, review the code architecturally (text-only, fine in main context):

1. Is it using Basecoat primitives correctly? Not rebuilding what Basecoat provides?
2. Are partials properly separated? Reusable components in `components/`, sections in `partials/`?
3. Is it following WordPress template hierarchy conventions?
4. Is the Tailwind usage correct? Using design tokens, not arbitrary values?
5. Does the HTML use semantic elements? (`<nav>`, `<main>`, `<section>`, `<article>`, `<footer>`)
6. Are interactive components using Alpine.js correctly? (hamburger menu, tabs, accordion)
7. Would this template be straightforward to make CMS-editable later? (clean structure, content not tangled with layout)

Write the review result to `build-state.json`.

**If architectural review fails** → go back to step 3, fix the code, increment `attempts`.

### 6. Advance

Once both reviews pass:
1. Update `build-state.json` with final status
2. Commit: `git add theme/ .treble/build-state.json && git commit -m "feat: implement {ComponentName}"`
3. Move to the next component in build order
4. Go back to step 1

## Stopping

- Stop after completing all components in the build order
- Stop if the user says stop
- Stop if you hit 3 failed attempts on a single component (mark as `"skipped"`, move on)

## Summary

After finishing (or stopping), tell the user:
- How many components implemented vs planned vs skipped
- Any components that failed visual or architectural review
- The local URL to view the site (e.g. `http://localhost:8080`)

**CMS nudge:**

```
Next step: make content editable.
Run /treble:cms to add WordPress CMS editability (Gutenberg blocks).
```
