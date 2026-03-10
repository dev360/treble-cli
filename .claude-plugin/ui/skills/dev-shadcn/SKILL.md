---
name: dev-shadcn
description: Build loop for React + shadcn/ui targets. Use this skill when /treble:dev determines the deployment target is Next.js or Astro, or when the user asks to build components with shadcn/ui.
version: 0.1.0
---

# /treble:dev-shadcn — Build Loop (React + shadcn/ui)

You are Treble's Build Agent for the **React + shadcn/ui** target. Your job is to implement components from `.treble/analysis.json`, following a strict code → visual review → architectural review loop.

**Primitive matching:** The planner uses shadcn/ui as the reference catalog for all targets. When `primitiveMatch.component` says `"Button"`, it means the shadcn/ui Button. In this target, use shadcn/ui directly — wrap/extend the matched primitive.

**CRITICAL:** ONLY use the `treble` CLI and local `.treble/` files for Figma data. Do NOT call the Figma API directly or use any Figma MCP server. All design data is on disk.

## Context Management

**NEVER read PNG/image files directly in the main conversation.** All image reading MUST happen inside subagents via the `Agent` tool. This prevents context window bloat that kills multi-component builds.

When you need to see a Figma reference or compare visuals, spawn a subagent to do the image work and return text results.

If you see "image dimension limit" errors, run `/compact` before continuing.

## Prerequisites

- `.treble/analysis.json` must exist (run `/treble:plan` first)
- `.treble/build-state.json` must exist
- The project should have a package.json and dev server configured
- `/treble:dev` Step 0 (project setup) should already be complete

## File Organization — Feature-Based Architecture

**This is the most important section.** Every component must go in the RIGHT place. Do NOT dump everything under `src/components/`.

### Directory structure

```
src/
├── components/
│   ├── ui/              # shadcn primitives ONLY — managed by `npx shadcn add`
│   │   ├── button.tsx   # shadcn Button
│   │   ├── card.tsx     # shadcn Card
│   │   └── ...
│   ├── common/          # reusable across 2+ features — branded, not generic
│   │   ├── Logo.tsx     # SVG logo (used in header AND footer)
│   │   ├── SocialLinks.tsx
│   │   └── ThemeToggle.tsx
│   └── layout/          # page-level layout shells
│       ├── PageLayout.tsx       # <Header /> + <main>{children}</main> + <Footer />
│       ├── SectionContainer.tsx # full-bleed wrapper + max-w container
│       ├── Header.tsx
│       └── Footer.tsx
├── features/
│   ├── home/            # one feature per page/domain
│   │   ├── components/
│   │   │   ├── HeroSection.tsx
│   │   │   ├── FeatureGrid.tsx
│   │   │   └── TestimonialCarousel.tsx
│   │   └── home-page.tsx        # main export — composes all sections
│   ├── about/
│   │   ├── components/
│   │   │   ├── TeamGrid.tsx
│   │   │   └── MissionStatement.tsx
│   │   └── about-page.tsx
│   └── pricing/
│       ├── components/
│       │   ├── PricingCard.tsx
│       │   └── PlanComparison.tsx
│       └── pricing-page.tsx
├── lib/                 # cn(), constants, shared utilities
└── app/ or pages/       # THIN route files — just mount features
    ├── page.tsx         # import { HomePage } from '@/features/home/home-page'
    └── about/page.tsx   # import { AboutPage } from '@/features/about/about-page'
```

### Placement decision — ask this for EVERY component

```
Is it a shadcn primitive (Button, Card, Input)?
  → src/components/ui/         (managed by shadcn CLI, don't touch)

Is it a page-level layout shell (Header, Footer, PageLayout, SectionContainer)?
  → src/components/layout/

Is it used across 2+ features AND is truly generic (Logo, SocialLinks, icons)?
  → src/components/common/

Everything else → src/features/{feature}/components/
```

**The test:** If you can't name 2+ features that use a component, it does NOT belong in `components/`. A `HeroSection` is NOT reusable — it's part of the `home` feature. A `PricingCard` is NOT reusable — it's part of the `pricing` feature.

### Mapping Figma frames to features

Each Figma frame (page design) typically maps to one feature:

| Figma Frame | Feature | Main export |
|-------------|---------|-------------|
| Homepage | `features/home/` | `home-page.tsx` |
| About | `features/about/` | `about-page.tsx` |
| Pricing | `features/pricing/` | `pricing-page.tsx` |
| Contact | `features/contact/` | `contact-page.tsx` |

**Shared sections** that appear across multiple page frames (header, footer, nav) go in `components/layout/`. Everything else stays in the feature.

### Route files are THIN

Route files (Next.js `app/page.tsx`, Astro `pages/index.astro`) do almost nothing:

```tsx
// app/page.tsx (Next.js)
import { HomePage } from '@/features/home/home-page'
export default function Page() {
  return <HomePage />
}
```

```astro
---
// pages/index.astro (Astro)
import { HomePage } from '../features/home/home-page'
import PageLayout from '../components/layout/PageLayout.astro'
---
<PageLayout><HomePage client:load /></PageLayout>
```

## Step 0: Project Bootstrap (run ONCE before the loop)

### 0a. Font Setup

Read `designSystem.fonts` from `analysis.json`. For EACH font:

1. **If `isCommercial: true`** — the font files are NOT available yet:
   - Use the `fallback` font stack as the primary font in CSS
   - Write a `@font-face` placeholder comment: `/* TODO: add licensed {family} .woff2 files */`
   - Configure Tailwind `fontFamily` to use the fallback: `heading: ["Inter", "system-ui", "sans-serif"]`
   - Add `font-display: swap` so it's ready for when the real font is added
   - **The build must look good with the fallback font.** Don't leave broken typography waiting for a font that may never arrive.

2. **If not commercial** (Google Font, open source):
   - Add `@import url('https://fonts.googleapis.com/css2?family={family}:wght@{weights}&display=swap')` to global CSS
   - Configure Tailwind `fontFamily` with the real font name + fallback

3. **For ALL fonts** — add metric-adjusted fallback to prevent layout shift:
   ```css
   @font-face {
     font-family: "{family}-fallback";
     src: local("Arial"); /* or closest system font */
     size-adjust: 100%;   /* adjust when real metrics are known */
     font-display: swap;
   }
   ```

### 0b. Responsive Foundation

Read `responsive` from `analysis.json`. Set up:

1. **Base layout wrapper** — create `src/components/layout/SectionContainer.tsx`:
   ```tsx
   interface SectionContainerProps {
     children: React.ReactNode
     className?: string
     as?: 'section' | 'div' | 'header' | 'footer'
   }

   export function SectionContainer({ children, className, as: Tag = 'section' }: SectionContainerProps) {
     return (
       <Tag className={cn("w-full", className)}>
         <div className="max-w-7xl mx-auto px-6">
           {children}
         </div>
       </Tag>
     )
   }
   ```

2. **Tailwind config** — ensure breakpoints match the analysis:
   - `sm: 640px`, `md: 768px`, `lg: 1024px`, `xl: 1280px` (Tailwind defaults are fine for most designs)

3. **Global CSS** — add fluid typography helpers if the analysis uses `clamp()`:
   ```css
   .fluid-heading-xl { font-size: clamp(2.25rem, 2vw + 1.5rem, 3.25rem); }
   .fluid-heading-lg { font-size: clamp(1.75rem, 1.5vw + 1rem, 2.5rem); }
   ```

### 0c. Create feature scaffolds

Read the Figma frames from analysis.json and create one feature per page:

```bash
# For each page frame:
mkdir -p src/features/{feature-name}/components
touch src/features/{feature-name}/{feature-name}-page.tsx
```

Create `src/components/layout/PageLayout.tsx` with Header + Footer shells.

Commit: `git commit -m "chore: scaffold feature directories"`

## The Loop

For each component in the build order:

### 1. Pick the next component

Read `.treble/build-state.json` and `.treble/analysis.json`. Find the next component where status is `"planned"`, following the `buildOrder` array.

If the user specified a component name, start there instead.

### 2. Gather context

Read the component's analysis entry from `analysis.json` (TEXT — this is fine in main context):
- `tier` — determines complexity (atom = simple, organism = composed)
- `primitiveMatch` — if set, wrap/extend the matched shadcn/ui primitive
- `composedOf` — import these (they should already be built)
- `figmaNodes` — which Figma layers this maps to
- `props`, `variants`, `tokens` — the component interface
- `filePath` — where to write the code
- `implementationNotes` — the detailed visual reproduction notes (THIS is your primary input)
- `referenceImages` — paths to screenshots (read these in a subagent, not here)

**For organisms/sections:** Also read the **section entry** from `pages.{page}.sections[]` in `analysis.json`. The section's `implementationNotes` contains layer composition strategy (prefixed with `LAYERS:`) that tells you how stacked Figma layers collapse into CSS. This is critical — multiple Figma layers often become a single HTML element with combined CSS. Do NOT create a separate `<div>` for every Figma layer. Follow the collapse strategy described in the notes.

**Use a subagent to examine reference images.** Spawn an Agent that reads the referenceImages PNGs and returns a text description of what it sees — colors, layout, spacing, typography. This keeps images out of the main context.

Read node properties for exact measurements (TEXT — fine in main context):
```bash
treble tree "{frameName}" --root "{nodeId}" --verbose
treble tree "{frameName}" --root "{nodeId}" --json
```

### 3. Code

Write the component following these rules. **File placement follows the feature-based architecture above — refer to the placement decision flowchart.**

**Atoms (shadcn wrappers / branded primitives):**
- If `primitiveMatch` is set — wrap/extend the matched shadcn/ui primitive
- Generic props — no hardcoded content
- Design tokens from the analysis, mapped to Tailwind classes
- File placement:
  - If it IS a shadcn primitive → `src/components/ui/` (use `npx shadcn add`)
  - If it wraps a shadcn primitive with brand styling AND is used across 2+ features → `src/components/common/`
  - If it's only used in one feature → `src/features/{feature}/components/`

**Organisms (sections):**
- Import their `composedOf` dependencies
- Layout matching the Figma structure (flexbox, grid)
- Accept content via props — sections are layout containers
- Use `SectionContainer` from `@/components/layout/SectionContainer` for full-bleed wrappers
- File at `src/features/{feature}/components/{ComponentName}.tsx`
- Sections are ALMOST NEVER reusable — they belong in their feature
- **Layer composition** — if the section's `implementationNotes` starts with `LAYERS:`, follow the collapse strategy exactly. Multiple Figma layers (image fill + gradient overlay + decorative shapes) become minimal HTML: typically a `relative` container, one or two `absolute inset-0` overlay divs, and content at `relative z-10`. Do NOT create a `<div>` per Figma layer.

**Layout components:**
- Header, Footer, PageLayout, SectionContainer
- File at `src/components/layout/{ComponentName}.tsx`

**Feature pages:**
- Import all sections from `./components/` in order
- Pass concrete content to sections
- Export a single named component: `export function HomePage() { ... }`
- File at `src/features/{feature}/{feature}-page.tsx`

**Route files (thin wrappers):**
- Next.js: `src/app/{route}/page.tsx` → `import { FeaturePage } from '@/features/{feature}/{feature}-page'`
- Astro: `src/pages/{route}.astro` → `import { FeaturePage } from '../features/{feature}/{feature}-page'`
- These files should be 3-5 lines. NO layout logic here.

**Assets — handle each `assetKind`:**

- **`svg-extract` (logos, icons, brand marks)** — NEVER try to reproduce these with CSS text styling:
  1. Render via `treble show "{nodeId}" --frame "{frameName}" --json` to get a screenshot
  2. Check if the Figma node contains VECTOR children — if so, note the node ID for SVG export
  3. Create a **real SVG placeholder component**:
     - If used across 2+ features (e.g. Logo) → `src/components/common/icons/{Name}.tsx`
     - If feature-specific → `src/features/{feature}/components/icons/{Name}.tsx`
     ```tsx
     // TODO: Replace with real SVG exported from Figma node {nodeId}
     // Export: Figma → select node → right-click → Copy as SVG → SVGO → paste here
     const Logo = ({ className, ...props }: React.SVGProps<SVGSVGElement>) => (
       <svg viewBox="0 0 {width} {height}" fill="none" className={className} {...props}>
         <rect width="{width}" height="{height}" rx="4" fill="#E5E7EB" />
         <text x="50%" y="50%" textAnchor="middle" dy=".3em" fill="#9CA3AF" fontSize="12">
           {Name}
         </text>
       </svg>
     )
     export { Logo }
     ```
  4. The placeholder must have the CORRECT dimensions (from Figma node width/height) and accept `className` + spread props
  5. When the user provides the real SVG, they just paste it inside the component — the interface stays the same

- **`icon-library`** → import from the matched icon library (e.g. `import { ArrowRight } from "lucide-react"`)

- **`image-extract`** → check `extractedImages` in analysis.json first:
  - If `extractedImages` has entries, copy from `.treble/figma/{slug}/assets/{file}` → `public/images/`
  - Use `<img src="/images/{file}">` in the component code
  - If no extracted images exist, fall back to `treble show` to render a screenshot, or use placeholder colors

**Responsive rules — apply to EVERY component:**

The Figma frame is a fixed-width desktop reference. Your code must work at ALL viewport sizes.

1. **Every section** must use the container pattern from `analysis.json → responsive`:
   - Full-bleed: outer `w-full`, inner wrapper with `max-w-7xl mx-auto px-6` (or whatever the analysis specifies)
   - NEVER hardcode `w-[1440px]` or any fixed pixel width on a section

2. **Grids collapse on mobile** — read the section's `responsive.mobileBehavior`:
   - 3-column → `grid-cols-1 md:grid-cols-2 lg:grid-cols-3`
   - 2-column asymmetric → `grid-cols-1 lg:grid-cols-[2fr_1fr]`
   - Side-by-side hero → `flex-col lg:flex-row`

3. **Typography scales** — use `clamp()` for headings 24px+:
   - `font-size: clamp(minRem, vw + rem, maxRem)` or Tailwind `text-[clamp(...)]`
   - Body text (14-18px) stays fixed — no clamp needed

4. **Navigation** — if the analysis says hamburger below 768px:
   - Desktop links: `hidden md:flex`
   - Hamburger button: `md:hidden`
   - Mobile menu: `useState` toggle, full-width dropdown or slide-in

5. **Spacing scales down** — hero padding, section gaps:
   - Use responsive prefixes: `py-12 md:py-20 lg:py-28`
   - Or fluid: `py-[clamp(3rem,5vw,7rem)]`

6. **Images are fluid** — always `w-full h-auto` or `object-cover` with constrained container

### 4. Visual Review (MANDATORY — via subagent)

You MUST do a real visual comparison after coding each organism/page component. This is not optional. "It renders without errors" is NOT a visual review.

**Step 4a: Capture implementation screenshot**

Spawn a `chrome-devtools-tester` subagent to screenshot the running dev server:

```
Navigate to localhost:{port} (or the specific route for this component).
Wait for the page to fully load (wait for network idle).
Take a full-page screenshot at 1440px width.
Save it to .treble/screenshots/{ComponentName}-impl.png
Also take section-level screenshots if the page is long — scroll to each section and capture it.
Return the file paths of all screenshots taken.
```

**Step 4b: Compare against Figma reference**

Spawn a `general-purpose` subagent that reads BOTH images and compares them:

```
You are doing a pixel-level visual comparison between a Figma design and a web implementation.

FIGMA REFERENCE: Read the file at {referenceImages[0]}
IMPLEMENTATION: Read the file at .treble/screenshots/{ComponentName}-impl.png

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

**Step 4c: Fix discrepancies**

If the comparison found issues (anything rated WRONG or CLOSE with significant discrepancies):
1. Fix the code based on the specific suggestions
2. Re-run step 4a and 4b
3. Max 3 attempts before marking as `"skipped"`

Write the visual review result to `build-state.json`:
```json
{
  "ComponentName": {
    "status": "implemented",
    "filePath": "src/features/{feature}/components/ComponentName.tsx",
    "generatedAt": "ISO-8601",
    "attempts": 1,
    "visualReview": {
      "passed": true,
      "discrepancies": [],
      "reviewedAt": "ISO-8601"
    }
  }
}
```

**SKIP visual review for atoms** (Button, Input, Badge) — they're too small to meaningfully screenshot. Only compare organisms and pages.

### 5. Architectural Review

After visual review passes, review the code architecturally (text-only, fine in main context):

1. **File placement correct?** Feature-specific components in `features/`, shared in `components/common/` or `layout/`? Nothing dumped in a flat `src/components/` grab bag?
2. Is it using primitives correctly? Not re-implementing what shadcn/ui provides?
3. Are props generic? No hardcoded strings that should be props?
4. Is the component properly composed? Using its `composedOf` dependencies?
5. Is it following React/Tailwind conventions?
6. Is the Tailwind usage correct? Using design tokens, not arbitrary values?
7. Is the component properly typed (TypeScript)?
8. **Feature page wired?** Is the component imported in its feature's `{feature}-page.tsx`?

Write the review result:
```json
{
  "ComponentName": {
    "codeReview": {
      "passed": true,
      "notes": [],
      "reviewedAt": "ISO-8601"
    }
  }
}
```

**If architectural review fails** → go back to step 3, fix the code, increment `attempts`.

### 6. Advance

Once both reviews pass:
1. Update `build-state.json` with final status
2. Commit: `git add src/features/{feature}/components/{ComponentName}.tsx .treble/build-state.json && git commit -m "feat({feature}): implement {ComponentName}"`
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
- What to do next (run the dev server, test, etc.)

**CMS nudge:** Read `.treble/build-state.json` → `buildConfig.compatibleCms` and tell the user:

```
Next step: make content editable.
Compatible CMS options: {list from buildConfig.compatibleCms, e.g. "Sanity, Prismic"}
Run /treble:cms to wire up your CMS.
```
