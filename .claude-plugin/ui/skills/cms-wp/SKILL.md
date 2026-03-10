---
name: cms-wp
description: Make a WordPress build editable with ACF fields, custom post types, and nav menus. Use this skill when /treble:cms determines the CMS platform is WordPress.
version: 0.1.0
---

# /treble:cms-wp — WordPress CMS Editability

You are Treble's CMS Agent for WordPress. The dev agent has already built pixel-perfect, fully styled pages with hardcoded content. Your job is to make those pages editable in WordPress — without breaking the visual fidelity.

**You are doing a refactor, not a rebuild.** The templates already look correct. You are replacing hardcoded strings and images with WordPress dynamic content so a marketing team can manage the site without touching code.

**CRITICAL:** Do NOT change layout, styling, or visual structure. If a page looks right before you touch it, it must look identical after. Every change you make is a content source swap — hardcoded value → dynamic field — nothing else.

## Prerequisites

- The dev-wp build must be complete — all pages implemented and visually reviewed
- `.treble/analysis.json` and `.treble/build-state.json` must exist
- Docker environment must be running (`docker-compose up -d`)

## Treble CLI Tools Available

You have access to `treble` CLI commands for Figma design data. Use these during visual verification (Step 5) to pull exact measurements when something looks off.

- `treble tree "{frameName}" --root "{nodeId}" --verbose` — node tree with visual properties (fills, text, layout, spacing)
- `treble tree "{frameName}" --root "{nodeId}" --json` — machine-readable node data
- `treble show "{nodeId}" --frame "{frameName}" --json` — render a screenshot of a specific Figma node

**On-disk design data:**
- `.treble/figma/{slug}/reference.png` — full-frame Figma screenshot (your ground truth)
- `.treble/figma/{slug}/nodes.json` — all Figma nodes with properties
- `.treble/figma/{slug}/image-map.json` — extracted source images (photos, logos)
- `.treble/figma/{slug}/assets/` — the actual image files
- `.treble/analysis.json` — component inventory, design system tokens, typography, colors

When a visual comparison flags a discrepancy, use `treble tree` to get the exact Figma values (font size, color, padding, gap) rather than eyeballing.

## Step 0: Choose CMS Approach

**Before doing anything else**, check if `.treble/cms-plan.json` already exists and has an `approach` field. If so, skip this step.

If no plan exists, ask the user which editing experience they want:

**Option A: Gutenberg Blocks (Recommended)**
Custom blocks for each section. Editors compose pages visually in the block editor — inline text editing, drag-and-drop reordering, media uploads in context. No separate "admin fields" page. Modern WordPress best practice.
- Best for: marketing teams who want visual editing, sites that will grow with new pages
- Requires: `@wordpress/scripts` build toolchain
- No ACF Pro dependency

**Option B: ACF Field Groups**
Each page section maps to ACF fields on the page edit screen. Editors fill out form fields (text inputs, image pickers, WYSIWYG editors) in a structured admin panel. Reliable, well-understood, fast to implement.
- Best for: simple sites with fixed page structures, teams familiar with ACF
- Requires: ACF Pro plugin (paid license)
- No JS build toolchain needed

**Option C: ACF Blocks (Hybrid)**
ACF fields rendered as visual block previews inside the Gutenberg editor. Editors get the block editor's drag-and-drop with ACF's structured field experience. Middle ground.
- Best for: teams that want some visual editing but prefer ACF's field model
- Requires: ACF Pro plugin (paid license)
- Minimal JS build toolchain

**Option D: Headless (API-only)**
WordPress serves as a content API only. The frontend stays as a separate app (Next.js, Astro, etc.) consuming the REST API or WPGraphQL. Full decoupling.
- Best for: teams with frontend developers, JAMstack/static sites, multi-channel content
- Requires: separate frontend deployment, WPGraphQL or REST API familiarity

Save the choice to `.treble/cms-plan.json`:
```json
{
  "version": 1,
  "approach": "gutenberg-blocks",
  "decidedAt": "ISO-8601"
}
```

Then follow the matching section below.

---

## Approach A: Gutenberg Blocks

### A0. Infrastructure

#### A0a. theme.json — Design System Bridge

Generate `theme.json` from `.treble/analysis.json → designSystem`. This makes the design tokens available in the block editor and locks down the color/typography palette so editors can't go off-brand.

```json
{
  "$schema": "https://schemas.wp.org/trunk/theme.json",
  "version": 3,
  "settings": {
    "color": {
      "palette": [
        { "slug": "dark-teal", "color": "#113333", "name": "Dark Teal" }
      ],
      "custom": false,
      "defaultPalette": false
    },
    "typography": {
      "fontFamilies": [
        {
          "fontFamily": "Georgia, 'Times New Roman', serif",
          "slug": "heading",
          "name": "Heading"
        }
      ],
      "fontSizes": [
        { "slug": "hero", "size": "clamp(2.5rem, 4vw, 3.75rem)", "name": "Hero" }
      ],
      "customFontSize": false
    },
    "spacing": { "units": ["px", "rem", "%", "vw"] },
    "layout": { "contentSize": "1280px", "wideSize": "1440px" }
  }
}
```

#### A0b. Block Build Toolchain

Set up the WordPress block build system:

```bash
# In the theme directory
npm init -y
npm install --save-dev @wordpress/scripts
```

Add to `package.json`:
```json
{
  "scripts": {
    "build": "wp-scripts build",
    "start": "wp-scripts start"
  }
}
```

File structure for blocks:
```
theme/
├── blocks/
│   ├── hero/
│   │   ├── block.json       # Block metadata + attributes
│   │   ├── index.js          # Client registration (edit + save)
│   │   ├── edit.js           # Editor component (React)
│   │   ├── render.php        # Frontend render (server-side)
│   │   └── editor.css        # Editor-only styles (optional)
│   ├── stats-bar/
│   │   ├── block.json
│   │   ├── index.js
│   │   ├── edit.js
│   │   └── render.php
│   └── ...
├── patterns/
│   └── homepage.php          # Pre-assembled page pattern
├── parts/
│   ├── header.html           # FSE template part
│   └── footer.html
├── build/                    # Compiled output (gitignored)
├── theme.json
├── functions.php
└── style.css
```

#### A0c. functions.php — Block Registration

```php
<?php
add_action('after_setup_theme', function () {
    add_theme_support('title-tag');
    add_theme_support('post-thumbnails');
    add_theme_support('custom-logo', [
        'height' => 60, 'width' => 200,
        'flex-height' => true, 'flex-width' => true,
    ]);
    add_theme_support('editor-styles');
    add_theme_support('html5', ['search-form', 'comment-form', 'gallery', 'caption']);

    register_nav_menus([
        'primary' => 'Primary Navigation',
        'footer'  => 'Footer Navigation',
    ]);
});

// Enqueue frontend styles
add_action('wp_enqueue_scripts', function () {
    wp_enqueue_style('treble-theme', get_theme_file_uri('assets/css/app.css'), [], '1.0.0');
});

// Enqueue editor styles so blocks look correct while editing
add_action('enqueue_block_editor_assets', function () {
    wp_enqueue_style('treble-editor', get_theme_file_uri('assets/css/app.css'), [], '1.0.0');
});

// Register all custom blocks from the blocks/ directory
add_action('init', function () {
    $blocks_dir = get_template_directory() . '/build/blocks';
    if (!is_dir($blocks_dir)) return;

    foreach (glob($blocks_dir . '/*/block.json') as $block_json) {
        register_block_type(dirname($block_json));
    }
});
```

#### A0d. Header & Footer as Template Parts

Use WordPress Full Site Editing template parts for header and footer:

**`parts/header.html`:**
```html
<!-- wp:group {"layout":{"type":"constrained","contentSize":"1440px"}} -->
<div class="wp-block-group">
  <!-- wp:navigation {"ref":PRIMARY_MENU_ID} /-->
</div>
<!-- /wp:group -->
```

For more complex headers with custom styling, create a `header` block instead — same pattern as section blocks.

### A1. Analyze Sections → Block Plan

For each page in `.treble/analysis.json → pages`, read the current template and classify each section:

**STATIC** — Decorative elements. Not a block. Baked into the section block's render.php.
- Background gradients, decorative SVGs, wave dividers
- These become internal details of the parent section block

**SECTION BLOCK** — A distinct page section with editable content.
- Each organism from the analysis becomes one custom block
- Block attributes map to the section's editable content (headings, body text, images, CTAs)
- The block's `render.php` is the dev agent's existing template with `$attributes` replacing hardcoded values

**INNER BLOCKS** — For sections containing repeatable items (card grids, team members, testimonials).
- The parent section block uses `<InnerBlocks>` to allow child blocks
- Each child (card, team member, etc.) is its own block
- `allowedBlocks` restricts what can be inserted
- `template` provides the default arrangement

**CUSTOM POST TYPE** — Content with its own identity that appears in lists/grids.
- Team members, case studies, blog posts, services
- Register the CPT with `show_in_rest: true` so Gutenberg works
- The listing section block uses a server-side `WP_Query` in `render.php`
- Individual CPT entries use the block editor with a block `template` for consistent structure

**PAGE PATTERN** — Pre-assembled full page layout using all the section blocks.
- Register as a block pattern so editors can create new pages from the template
- Auto-insert for specific page templates via CPT `template` property

Write the block plan to `.treble/cms-plan.json`:

```json
{
  "version": 1,
  "approach": "gutenberg-blocks",
  "decidedAt": "ISO-8601",
  "globals": {
    "navMenus": ["primary", "footer"],
    "templateParts": ["header", "footer"],
    "customPostTypes": [
      {
        "name": "physician",
        "singular": "Physician",
        "plural": "Physicians",
        "blockTemplate": ["theme/physician-details"],
        "fields": ["photo", "name", "credentials", "specialty", "bio"],
        "hasArchive": true
      }
    ]
  },
  "blocks": [
    {
      "name": "theme/hero",
      "analysisComponent": "HeroSection",
      "attributes": [
        {"name": "heading", "type": "string", "source": "rich-text"},
        {"name": "subtext", "type": "string", "source": "rich-text"},
        {"name": "backgroundImage", "type": "object", "source": "media-upload"},
        {"name": "ctaPrimary", "type": "object", "source": "link-control"},
        {"name": "ctaSecondary", "type": "object", "source": "link-control"}
      ],
      "usesInnerBlocks": false
    },
    {
      "name": "theme/stats-bar",
      "analysisComponent": "StatsBar",
      "attributes": [
        {"name": "sectionTitle", "type": "string", "source": "rich-text"}
      ],
      "usesInnerBlocks": true,
      "allowedBlocks": ["theme/stat-item"],
      "defaultTemplate": [
        ["theme/stat-item", {"value": "37", "label": "years avg client tenure"}],
        ["theme/stat-item", {"value": "$1+B", "label": "in revenue recovery"}],
        ["theme/stat-item", {"value": "$18M", "label": "fewer denied claims"}]
      ]
    }
  ],
  "patterns": [
    {
      "name": "theme/homepage",
      "title": "Homepage",
      "blocks": ["theme/hero", "theme/stats-bar", "theme/process-section", "..."]
    }
  ]
}
```

**Show this plan to the user and get confirmation before proceeding.**

### A2. Create Section Blocks

For each block in the plan, create three files:

#### block.json — Block Metadata

```json
{
  "apiVersion": 3,
  "$schema": "https://schemas.wp.org/trunk/block.json",
  "name": "theme/hero",
  "title": "Hero Section",
  "category": "theme",
  "description": "Full-width hero with photo background, overlay, heading, and CTAs",
  "supports": {
    "html": false,
    "align": ["full", "wide"]
  },
  "attributes": {
    "heading": { "type": "string", "default": "" },
    "subtext": { "type": "string", "default": "" },
    "backgroundImage": {
      "type": "object",
      "default": { "id": 0, "url": "", "alt": "" }
    },
    "ctaPrimaryText": { "type": "string", "default": "Get Started" },
    "ctaPrimaryUrl": { "type": "string", "default": "#" },
    "ctaSecondaryText": { "type": "string", "default": "Learn More" },
    "ctaSecondaryUrl": { "type": "string", "default": "#" }
  },
  "editorScript": "file:./index.js",
  "render": "file:./render.php"
}
```

#### edit.js — Editor Component

The edit component gives editors inline visual editing. Use `RichText` for text, `MediaUpload` for images, `URLInput` / sidebar `TextControl` for links.

```jsx
import { useBlockProps, RichText, MediaUpload, MediaUploadCheck, InspectorControls } from '@wordpress/block-editor';
import { PanelBody, TextControl, Button } from '@wordpress/components';
import { __ } from '@wordpress/i18n';

export default function Edit({ attributes, setAttributes }) {
    const { heading, subtext, backgroundImage, ctaPrimaryText, ctaPrimaryUrl, ctaSecondaryText, ctaSecondaryUrl } = attributes;

    return (
        <>
            <InspectorControls>
                <PanelBody title={__('Background Image')}>
                    <MediaUploadCheck>
                        <MediaUpload
                            onSelect={(media) => setAttributes({ backgroundImage: { id: media.id, url: media.url, alt: media.alt } })}
                            allowedTypes={['image']}
                            value={backgroundImage.id}
                            render={({ open }) => (
                                <Button onClick={open} variant="secondary">
                                    {backgroundImage.url ? __('Replace Image') : __('Select Image')}
                                </Button>
                            )}
                        />
                    </MediaUploadCheck>
                    {backgroundImage.url && (
                        <img src={backgroundImage.url} alt="" style={{ maxWidth: '100%', marginTop: 8 }} />
                    )}
                </PanelBody>
                <PanelBody title={__('CTAs')}>
                    <TextControl label={__('Primary Button Text')} value={ctaPrimaryText} onChange={(v) => setAttributes({ ctaPrimaryText: v })} />
                    <TextControl label={__('Primary Button URL')} value={ctaPrimaryUrl} onChange={(v) => setAttributes({ ctaPrimaryUrl: v })} />
                    <TextControl label={__('Secondary Link Text')} value={ctaSecondaryText} onChange={(v) => setAttributes({ ctaSecondaryText: v })} />
                    <TextControl label={__('Secondary Link URL')} value={ctaSecondaryUrl} onChange={(v) => setAttributes({ ctaSecondaryUrl: v })} />
                </PanelBody>
            </InspectorControls>

            <div {...useBlockProps({ className: 'hero-section' })}
                 style={{ backgroundImage: backgroundImage.url ? `url(${backgroundImage.url})` : undefined }}>
                <RichText
                    tagName="h1"
                    value={heading}
                    onChange={(v) => setAttributes({ heading: v })}
                    placeholder={__('Hero heading...')}
                    className="hero-heading"
                />
                <RichText
                    tagName="p"
                    value={subtext}
                    onChange={(v) => setAttributes({ subtext: v })}
                    placeholder={__('Supporting text...')}
                    className="hero-subtext"
                />
                <div className="hero-ctas">
                    <span className="btn btn-primary">{ctaPrimaryText || 'Get Started'}</span>
                    <span className="btn btn-link">{ctaSecondaryText || 'Learn More'} →</span>
                </div>
            </div>
        </>
    );
}
```

**Key patterns for edit.js:**
- `RichText` for all editable text (heading, body, quote) — enables inline editing
- `MediaUpload` + `MediaUploadCheck` in `InspectorControls` sidebar for images
- `TextControl` in `InspectorControls` for URLs and simple strings
- `useBlockProps()` for proper block wrapper attributes
- Editor should look roughly like the final output — enqueue the theme CSS in the editor

#### render.php — Frontend Output

This is the dev agent's existing template HTML with `$attributes` replacing hardcoded values. **Keep the exact same Tailwind classes and HTML structure.**

```php
<?php
$heading = $attributes['heading'] ?? '';
$subtext = $attributes['subtext'] ?? '';
$bg = $attributes['backgroundImage'] ?? [];
$cta_text = $attributes['ctaPrimaryText'] ?? 'Get Started';
$cta_url = $attributes['ctaPrimaryUrl'] ?? '#';
$cta2_text = $attributes['ctaSecondaryText'] ?? 'Learn More';
$cta2_url = $attributes['ctaSecondaryUrl'] ?? '#';

$wrapper = get_block_wrapper_attributes(['class' => 'relative w-full h-[800px] overflow-hidden']);
?>

<section <?php echo $wrapper; ?>>
    <?php if (!empty($bg['url'])): ?>
        <img src="<?php echo esc_url($bg['url']); ?>" alt="<?php echo esc_attr($bg['alt'] ?? ''); ?>"
             class="absolute inset-0 w-full h-full object-cover" loading="eager" />
    <?php endif; ?>

    <div class="absolute inset-0 bg-gradient-to-r from-[#113333] via-[#113333]/80 to-transparent"></div>

    <div class="relative z-10 max-w-7xl mx-auto px-6 flex flex-col justify-center h-full">
        <?php if ($heading): ?>
            <h1 class="text-[clamp(2.5rem,4vw,3.75rem)] font-light font-serif text-white max-w-[591px] leading-tight">
                <?php echo wp_kses_post($heading); ?>
            </h1>
        <?php endif; ?>

        <?php if ($subtext): ?>
            <p class="text-base text-white/70 max-w-[437px] mt-6 leading-relaxed">
                <?php echo wp_kses_post($subtext); ?>
            </p>
        <?php endif; ?>

        <div class="flex items-center gap-4 mt-8">
            <a href="<?php echo esc_url($cta_url); ?>" class="h-10 px-6 rounded-lg bg-[#CDB07A] text-[#25282A] text-[15px] font-normal inline-flex items-center shadow-[inset_0_-2px_4px_rgba(0,0,0,0.1)]">
                <?php echo esc_html($cta_text); ?>
            </a>
            <a href="<?php echo esc_url($cta2_url); ?>" class="text-white text-[15px] inline-flex items-center gap-2">
                <?php echo esc_html($cta2_text); ?> →
            </a>
        </div>
    </div>
</section>
```

**Rules for render.php:**
1. **Always escape output.** `esc_html()` for text, `esc_url()` for URLs, `esc_attr()` for attributes, `wp_kses_post()` for RichText content.
2. **Keep exact same HTML structure and CSS classes** from the dev agent's template.
3. **Wrap in conditionals** — if a field is empty, don't render empty markup.
4. **Use `get_block_wrapper_attributes()`** for the outermost element — this adds WordPress block classes.

#### InnerBlocks Pattern (for repeatable items)

For sections with repeatable children (stats, cards, team grid):

**Parent block edit.js:**
```jsx
import { useBlockProps, InnerBlocks } from '@wordpress/block-editor';

const ALLOWED_BLOCKS = ['theme/stat-item'];
const TEMPLATE = [
    ['theme/stat-item', { value: '37', label: 'years avg client tenure' }],
    ['theme/stat-item', { value: '$1+B', label: 'in revenue recovery' }],
    ['theme/stat-item', { value: '$18M', label: 'fewer denied claims' }],
];

export default function Edit() {
    return (
        <div {...useBlockProps()}>
            <InnerBlocks allowedBlocks={ALLOWED_BLOCKS} template={TEMPLATE} />
        </div>
    );
}
```

**Parent block render.php:**
```php
<section <?php echo get_block_wrapper_attributes(['class' => 'w-full py-16']); ?>>
    <div class="max-w-7xl mx-auto px-6 grid grid-cols-1 md:grid-cols-3 gap-8">
        <?php echo $content; ?>
    </div>
</section>
```

The `$content` variable contains the rendered InnerBlocks children.

### A3. Register Custom Post Types

For CPTs (team members, case studies, etc.):

```php
add_action('init', function () {
    register_post_type('physician', [
        'labels' => ['name' => 'Physicians', 'singular_name' => 'Physician'],
        'public' => true,
        'has_archive' => true,
        'menu_icon' => 'dashicons-groups',
        'supports' => ['title', 'editor', 'thumbnail'],
        'show_in_rest' => true,
        'template' => [
            ['theme/physician-details'],
        ],
        'template_lock' => 'all',
    ]);
});
```

The `template` + `template_lock` ensures every new physician entry starts with the right block structure and editors can't remove it.

### A4. Register Page Patterns

Create a pre-assembled homepage pattern so editors can create a new homepage with one click:

**`patterns/homepage.php`:**
```php
<?php
/**
 * Title: Homepage
 * Slug: theme/homepage
 * Categories: pages
 * Post Types: page
 * Block Types: core/post-content
 */
?>
<!-- wp:theme/hero {"heading":"Physician-Powered Revenue Accuracy","subtext":"Supporting text here...","backgroundImage":{"url":"","alt":""}} /-->
<!-- wp:theme/stats-bar -->
<!-- wp:theme/stat-item {"value":"37","label":"years avg client tenure"} /-->
<!-- wp:theme/stat-item {"value":"$1+B","label":"in revenue recovery"} /-->
<!-- wp:theme/stat-item {"value":"$18M","label":"fewer denied claims"} /-->
<!-- /wp:theme/stats-bar -->
<!-- wp:theme/process-section /-->
<!-- wp:theme/services-section /-->
<!-- ... -->
```

### A5. Build and Populate Content

```bash
# Build the blocks
cd theme && npm run build

# Create the homepage
docker-compose exec wordpress wp post create \
  --post_type=page \
  --post_title="Home" \
  --post_status=publish \
  --allow-root

# Set as front page
docker-compose exec wordpress wp option update show_on_front page --allow-root
docker-compose exec wordpress wp option update page_on_front $(docker-compose exec wordpress wp post list --post_type=page --name=home --field=ID --allow-root) --allow-root

# Create navigation menus
docker-compose exec wordpress wp menu create "Primary" --allow-root
docker-compose exec wordpress wp menu location assign "Primary" primary --allow-root
```

Then open the block editor in the browser — the homepage pattern should be available when editing the Home page. The editor uses `RichText` for inline editing, and the sidebar has media/link controls.

---

## Approach B: ACF Field Groups

### B0. Infrastructure

Install ACF Pro:
```bash
docker-compose exec wordpress wp plugin install advanced-custom-fields-pro --activate --allow-root
```
If that fails (ACF Pro requires a license), tell the user to install manually.

Enable ACF Local JSON for version control:
```php
add_filter('acf/settings/save_json', fn() => get_stylesheet_directory() . '/acf-json');
add_filter('acf/settings/load_json', function ($paths) {
    $paths[] = get_stylesheet_directory() . '/acf-json';
    return $paths;
});
```

Set up `theme.json`, `functions.php`, header/footer with `wp_nav_menu()` and ACF options pages — same as Gutenberg approach for infrastructure, but add:

```php
if (function_exists('acf_add_options_page')) {
    acf_add_options_page([
        'page_title' => 'Site Settings',
        'menu_title' => 'Site Settings',
        'menu_slug'  => 'site-settings',
        'capability' => 'edit_posts',
    ]);
}
```

### B1. Classify Sections

For each section, decide:
- **FIELD GROUP** — fixed structure, editable content (hero, stats, specific testimonial)
- **REPEATER** — list of identical items (card grids, feature lists)
- **FLEXIBLE CONTENT** — reusable section block for page builder (CTA banner, text+image)
- **CUSTOM POST TYPE** — independent content entities (team members, case studies)

### B2. Register ACF Fields

Use `acf_add_local_field_group()` in PHP. Key field type mapping:

| Content | ACF Type | Notes |
|---------|----------|-------|
| Headline | `text` | Single line |
| Body text | `textarea` or `wysiwyg` | `textarea` for plain, `wysiwyg` for formatted |
| CTA / link | `link` | Returns `[url, title, target]` array |
| Image | `image` | Return format: `array` |
| Stat number | `text` | NOT `number` — stats have symbols ($, +, M, B) |
| Repeatable items | `repeater` | Sub-fields for each property |
| Page builder sections | `flexible_content` | Each layout = section type |

### B3. Rewrite Templates

Replace hardcoded content with `get_field()` calls. Surgical replacement only.

**Before:**
```php
<p class="text-5xl font-bold">37</p>
```

**After:**
```php
<?php $stats = get_field('stats'); ?>
<?php if ($stats): ?>
  <?php foreach ($stats as $stat): ?>
    <p class="text-5xl font-bold"><?= esc_html($stat['number']) ?></p>
  <?php endforeach; ?>
<?php endif; ?>
```

**Rules:** Always escape output. Wrap in `if` checks. Keep exact same HTML/CSS. Use `get_field('field', 'option')` for globals.

### B4. Populate Content via WP-CLI

```bash
PAGE_ID=$(docker-compose exec wordpress wp post list --post_type=page --name=home --field=ID --allow-root)
docker-compose exec wordpress wp eval "update_field('hero_heading', 'Your Heading', $PAGE_ID);" --allow-root
```

---

## Approach C: ACF Blocks (Hybrid)

Same as Approach B for field registration, but use ACF's block registration:

```php
acf_register_block_type([
    'name'            => 'hero',
    'title'           => 'Hero Section',
    'render_template' => 'blocks/hero/render.php',
    'category'        => 'theme',
    'icon'            => 'cover-image',
    'mode'            => 'preview',
    'supports'        => ['align' => ['full', 'wide']],
]);
```

Each block gets a `render.php` that uses `get_field()` instead of `$attributes`. Editors see a visual preview in the block editor with ACF fields in the sidebar.

---

## Approach D: Headless (API-only)

Register CPTs and taxonomies with `show_in_rest: true`. Install WPGraphQL if preferred over REST. Content is managed in wp-admin, but the frontend is a separate deployment. The dev agent's templates become React/Astro components that fetch from the API.

This approach is beyond the scope of template refactoring — tell the user they need a separate frontend build and point them to WPGraphQL + Next.js or Astro documentation.

---

## Step 5: Visual Verification (Two-Tier, MANDATORY)

**This step applies to ALL approaches.** You MUST do a real visual comparison after all templates are rewritten and content is populated. This is not optional. "It renders without errors" is NOT a visual review.

You run TWO comparisons — one to catch template rewrite regressions, one to catch accumulated drift from the original Figma design.

### 5a. Capture post-CMS screenshot

Spawn a `chrome-devtools-tester` subagent to screenshot the WordPress site:

```
Navigate to localhost:{port} (or the specific route for this page).
Wait for the page to fully load (wait for network idle).
Take a full-page screenshot at 1440px width.
Save it to .treble/screenshots/{PageName}-cms.png
Also take section-level screenshots if the page is long — scroll to each section and capture it.
Return the file paths of all screenshots taken.
```

### 5b. Tier 1 — Regression check (post-CMS vs pre-CMS)

Spawn a `general-purpose` subagent that reads BOTH screenshots and compares them:

```
You are doing a pixel-level visual comparison between the pre-CMS and post-CMS builds. These should be nearly identical — the CMS step only swaps content sources, not layout or styling.

PRE-CMS: Read the file at .treble/screenshots/{PageName}-impl.png
POST-CMS: Read the file at .treble/screenshots/{PageName}-cms.png

Compare these two images section by section. For EACH visual section (nav, hero, features, footer, etc.), report:

1. LAYOUT — Is the structure correct? Flex direction, element order, alignment?
2. SPACING — Are margins, padding, gaps visually matching?
3. COLORS — Do backgrounds, text colors, borders match?
4. TYPOGRAPHY — Font sizes, weights, line-heights look right?
5. SHAPES — Border radius, shadows, decorative elements?
6. IMAGES/ICONS — Are placeholders roughly the right size/position?

Be HARSH. Flag every difference you see, no matter how small. Rate each section: MATCH / CLOSE / WRONG.

Return JSON:
{
  "overall": "MATCH|CLOSE|WRONG",
  "sections": [
    {
      "name": "NavBar",
      "rating": "WRONG",
      "discrepancies": ["nav links now wrapped in <ul><li>, showing bullet points", "nav spacing changed"],
      "suggestions": ["Add custom Walker class to strip default list styling", "Add gap-[27px] to walker output"]
    }
  ]
}
```

**Fix regressions:** If issues found, fix the template, re-screenshot, re-compare. Max 3 attempts before marking as `"skipped"`.

**Common regressions:**
- `wp_nav_menu()` different markup → custom Walker class
- ACF/block image fields different sizes → explicit width/height
- WYSIWYG `<p>` wrapping → use textarea or strip wrapping
- Missing content → check field names / attribute names
- Block wrapper adds extra div → adjust CSS selectors

### 5c. Tier 2 — Figma fidelity check (post-CMS vs Figma reference)

Spawn a `general-purpose` subagent that reads BOTH images and compares them:

```
You are doing a pixel-level visual comparison between a Figma design and a web implementation.

FIGMA REFERENCE: Read the file at {referenceImages[0]}
IMPLEMENTATION: Read the file at .treble/screenshots/{PageName}-cms.png

Compare these two images section by section. For EACH visual section (nav, hero, features, footer, etc.), report:

1. LAYOUT — Is the structure correct? Flex direction, element order, alignment?
2. SPACING — Are margins, padding, gaps visually matching?
3. COLORS — Do backgrounds, text colors, borders match?
4. TYPOGRAPHY — Font sizes, weights, line-heights look right?
5. SHAPES — Border radius, shadows, decorative elements?
6. IMAGES/ICONS — Are placeholders roughly the right size/position?

Be HARSH. Flag every difference you see, no matter how small. Rate each section: MATCH / CLOSE / WRONG.

Return JSON:
{
  "overall": "MATCH|CLOSE|WRONG",
  "sections": [
    {
      "name": "Hero",
      "rating": "CLOSE",
      "discrepancies": ["heading font too small — Figma shows ~56px, impl looks ~36px", "CTA button missing gold background"],
      "suggestions": ["Change text-3xl to text-5xl", "Add bg-accent to button"]
    }
  ]
}
```

If no `referenceImages` exist for the page, use the full-frame reference: `.treble/figma/{slug}/reference.png`

### 5d. Fix Figma discrepancies

If the comparison found issues (anything rated WRONG or CLOSE with significant discrepancies):

1. **Use `treble tree` to get exact values** — don't guess. Pull the real font size, color, padding, gap from the Figma node data.
   ```bash
   treble tree "{frameName}" --root "{nodeId}" --verbose
   ```

2. **Fix the CSS/template** — adjust Tailwind classes, spacing values, colors. Change ONLY styling, not content source.

3. **Re-run step 5a and 5c**. Max 3 attempts before marking as `"skipped"`.

Write the visual review result to `build-state.json`:
```json
{
  "PageName": {
    "status": "cms-complete",
    "filePath": "templates/template-home.php",
    "generatedAt": "ISO-8601",
    "attempts": 1,
    "visualReview": {
      "passed": true,
      "discrepancies": [],
      "acceptedDiscrepancies": ["Commercial font fallback — Georgia vs Canela Text"],
      "reviewedAt": "ISO-8601"
    },
    "regressionCheck": {
      "passed": true,
      "discrepancies": [],
      "reviewedAt": "ISO-8601"
    }
  }
}
```

## Step 6: Write Editorial Guide

Create `theme/EDITORIAL-GUIDE.md` explaining in plain language:

1. How to edit each page — where to find fields/blocks in the WordPress admin
2. How to manage CPTs (team members, case studies) — add/edit/reorder
3. How to compose new pages (for Gutenberg: using block patterns; for ACF: using flexible content)
4. How to update header/footer — nav menus, logo, CTAs
5. What NOT to touch — don't edit theme files, don't install random plugins

## Step 7: Commit

```bash
git add theme/ .treble/cms-plan.json .treble/build-state.json
git commit -m "feat: add CMS editability for {PageName} ({approach})"
```

## Summary

After finishing, tell the user:
- Which approach was used and why
- Which pages are now editable
- Which custom post types were created
- How to access the editor: `http://localhost:{wpPort}/wp-admin`
- Where the editorial guide is: `theme/EDITORIAL-GUIDE.md`
- **Next steps:** Set up forms (Gravity Forms / Contact Form 7), configure SEO (Yoast), review security
