---
description: Make a React+shadcn build editable via Prismic Slice Machine
arguments:
  - name: page
    description: Specific page/feature to make editable (optional, does all if omitted)
    required: false
---

# Prismic CMS Integration (React + shadcn/ui)

You are Treble's CMS Agent for Prismic. The dev agent has already built pixel-perfect pages using the feature-based architecture (`src/features/*/`). Your job is to make content editable via Prismic — without breaking visual fidelity.

**You are doing a refactor, not a rebuild.** Components already look correct. You are swapping hardcoded strings and images for Prismic fields — nothing else changes.

**CRITICAL:** Do NOT change layout, styling, or visual structure. Every change is a content source swap — hardcoded value → Prismic field.

## Prerequisites

- The dev build must be complete — pages implemented and visually reviewed
- `.treble/analysis.json` and `.treble/build-state.json` must exist
- Project must be Next.js (App Router) — Prismic's primary target

## Treble CLI Tools

Use these during visual verification to pull exact Figma measurements when something looks off:

- `treble tree "{frameName}" --root "{nodeId}" --verbose` — node tree with visual properties
- `treble tree "{frameName}" --root "{nodeId}" --json` — machine-readable node data
- `treble show "{nodeId}" --frame "{frameName}" --json` — screenshot of a specific Figma node

**On-disk design data:**
- `.treble/figma/{slug}/reference.png` — full-frame Figma screenshot (ground truth)
- `.treble/figma/{slug}/nodes.json` — all Figma nodes with properties
- `.treble/analysis.json` — component inventory, design system, tokens

## Step 0: Prismic Setup

### 0a. Check for existing setup

If `slicemachine.config.json` exists, skip to Step 1.

### 0b. Account check

Ask the user: **"Do you have a Prismic repository? If not, create a free one at prismic.io/dashboard and tell me the repository name."**

The agent cannot create the Prismic repo — this is the ONE manual step.

### 0c. Initialize Slice Machine

```bash
npx @slicemachine/init@latest
```

This scaffolds:
- `slicemachine.config.json` — config
- `src/prismicio.ts` — client factory
- `src/slices/` — slice directory
- `customtypes/` — page type models
- `src/app/slice-simulator/page.tsx` — local preview
- `src/app/api/preview/route.ts` — draft mode routes

### 0d. Install packages (if init didn't)

```bash
npm install @prismicio/client @prismicio/react @prismicio/next
npm install -D @slicemachine/adapter-next slice-machine-ui
```

### 0e. Configure routes in `prismicio.ts`

Read the features from the build to determine routes:

```ts
const routes: Route[] = [
  { type: "page", uid: "home", path: "/" },
  { type: "page", path: "/:uid" },
  // Add one entry per feature that maps to a page
]
```

### 0f. Add preview support to root layout

```tsx
// src/app/layout.tsx
import { PrismicPreview } from "@prismicio/next"
import { repositoryName } from "@/prismicio"

// Add at the end of <body>:
<PrismicPreview repositoryName={repositoryName} />
```

### 0g. Create the page custom type

Write `customtypes/page/index.json`:

```json
{
  "id": "page",
  "label": "Page",
  "format": "page",
  "repeatable": true,
  "status": true,
  "json": {
    "Main": {
      "uid": { "type": "UID", "config": { "label": "URL Slug" } },
      "title": {
        "type": "StructuredText",
        "config": { "label": "Page Title", "single": "heading1" }
      },
      "slices": {
        "type": "Slices",
        "fieldset": "Slice Zone",
        "config": { "choices": {} }
      }
    },
    "SEO": {
      "meta_title": { "type": "Text", "config": { "label": "Meta Title" } },
      "meta_description": { "type": "Text", "config": { "label": "Meta Description" } },
      "meta_image": { "type": "Image", "config": { "label": "OG Image", "constraint": {} } }
    }
  }
}
```

Commit: `git commit -m "chore: initialize Prismic Slice Machine"`

---

## The Loop

For each section component in the build order:

### 1. Pick the next component

Read `.treble/build-state.json`. Find the next organism/section where `cmsStatus` is not `"done"`. Skip atoms — they don't need CMS wiring (they receive data via props from their parent section).

### 2. Analyze what's editable

Read the component's source code. Identify every hardcoded value:

- **Text strings** → `StructuredText` (headings) or `Text` (plain strings)
- **Images** → `Image` field
- **Links/URLs** → `Link` field (with `allowText: true` if the link has a label)
- **Repeating items** (cards, features, testimonials) → `items` zone (repeatable group)
- **Boolean toggles** (show/hide sections) → `Boolean` field
- **Select options** (theme, layout variant) → `Select` field

### 3. Create the slice model

Write `src/slices/{SliceName}/model.json`:

```json
{
  "id": "hero_section",
  "type": "SharedSlice",
  "name": "Hero Section",
  "description": "Main hero banner",
  "variations": [
    {
      "id": "default",
      "name": "Default",
      "description": "Default hero layout",
      "imageUrl": "",
      "primary": {
        "heading": {
          "type": "StructuredText",
          "config": { "label": "Heading", "single": "heading1,strong,em" }
        },
        "subheading": {
          "type": "StructuredText",
          "config": { "label": "Subheading", "multi": "paragraph,strong,em" }
        },
        "cta_link": {
          "type": "Link",
          "config": { "label": "CTA Button", "allowText": true }
        },
        "background_image": {
          "type": "Image",
          "config": { "label": "Background Image", "constraint": {} }
        }
      },
      "items": {}
    }
  ]
}
```

**Field type reference:**
- `Text` — plain string: `{ "type": "Text", "config": { "label": "...", "placeholder": "..." } }`
- `StructuredText` (single heading): `{ "type": "StructuredText", "config": { "single": "heading1,strong,em" } }`
- `StructuredText` (rich body): `{ "type": "StructuredText", "config": { "multi": "paragraph,strong,em,hyperlink,list-item" } }`
- `Image`: `{ "type": "Image", "config": { "label": "...", "constraint": {} } }`
- `Link`: `{ "type": "Link", "config": { "label": "...", "allowText": true } }`
- `Select`: `{ "type": "Select", "config": { "options": ["light","dark"], "default_value": "light" } }`
- `Boolean`: `{ "type": "Boolean", "config": { "default_value": false } }`

For **repeating items** (feature cards, testimonials), put fields in the `items` zone instead of `primary`.

### 4. Create the slice wrapper component

Write `src/slices/{SliceName}/index.tsx`:

```tsx
import { type FC } from "react"
import { type Content, isFilled, asText, asLink } from "@prismicio/client"
import { PrismicRichText, type SliceComponentProps } from "@prismicio/react"
import { PrismicNextImage, PrismicNextLink } from "@prismicio/next"

// Import the EXISTING feature component — do NOT rewrite it
import { HeroSection } from "@/features/home/components/HeroSection"

type Props = SliceComponentProps<Content.HeroSectionSlice>

const HeroSectionSlice: FC<Props> = ({ slice }) => {
  return (
    <section data-slice-type={slice.slice_type} data-slice-variation={slice.variation}>
      <HeroSection
        headline={isFilled.richText(slice.primary.heading) ? asText(slice.primary.heading) : ""}
        subheadline={isFilled.richText(slice.primary.subheading) ? asText(slice.primary.subheading) : undefined}
        ctaLabel={slice.primary.cta_link?.text ?? "Learn More"}
        ctaHref={isFilled.link(slice.primary.cta_link) ? (asLink(slice.primary.cta_link) ?? "#") : "#"}
        imageSrc={slice.primary.background_image?.url ?? ""}
        imageAlt={slice.primary.background_image?.alt ?? ""}
      />
    </section>
  )
}

export default HeroSectionSlice
```

**Key pattern:** The slice wrapper is a THIN adapter. It extracts Prismic field data and passes it as props to the existing component. The existing component does NOT change.

**Always guard with `isFilled`:** `isFilled.richText()`, `isFilled.image()`, `isFilled.link()`, `isFilled.keyText()`.

For **repeating items**, map `slice.items`:
```tsx
features={slice.items.map((item, i) => ({
  title: isFilled.keyText(item.title) ? item.title : "",
  description: isFilled.richText(item.description) ? asText(item.description) : "",
  icon: item.icon_name ?? "star",
}))}
```

### 5. Register the slice

Add the slice ID to the page custom type's slice zone choices in `customtypes/page/index.json`:

```json
"choices": {
  "hero_section": { "type": "SharedSlice" },
  "feature_grid": { "type": "SharedSlice" }
}
```

Update `src/slices/index.ts` (or let Slice Machine regenerate it):
```ts
import dynamic from "next/dynamic"
export const components = {
  hero_section: dynamic(() => import("./HeroSection")),
  feature_grid: dynamic(() => import("./FeatureGrid")),
}
```

### 6. Update the page to use SliceZone

Replace the feature page's hardcoded section composition with a Prismic SliceZone:

**Before** (`src/features/home/home-page.tsx`):
```tsx
export function HomePage() {
  return (
    <>
      <HeroSection headline="Welcome" ... />
      <FeatureGrid features={[...]} />
      <Testimonials items={[...]} />
    </>
  )
}
```

**After** (`src/app/page.tsx`):
```tsx
import { SliceZone } from "@prismicio/react"
import { createClient } from "@/prismicio"
import { components } from "@/slices"

export default async function HomePage() {
  const client = createClient()
  const page = await client.getByUID("page", "home")
  return <SliceZone slices={page.data.slices} components={components} />
}
```

The feature page file (`home-page.tsx`) can stay as a non-CMS fallback, or be removed once slices are live.

### 7. Visual Review (MANDATORY)

Same two-tier review as the dev build. This is not optional.

**Tier 1: Regression check (pre-CMS vs post-CMS)**

Spawn a `chrome-devtools-tester` subagent:
```
Navigate to localhost:3000 (or the page route).
Wait for full load. Screenshot at 1440px width.
Save to .treble/screenshots/{PageName}-cms.png
```

Spawn a `general-purpose` subagent to compare:
```
Compare these two images section by section:
PRE-CMS: .treble/screenshots/{PageName}-impl.png
POST-CMS: .treble/screenshots/{PageName}-cms.png

For EACH section, report:
1. LAYOUT — structure, flex direction, alignment
2. SPACING — margins, padding, gaps
3. COLORS — backgrounds, text, borders
4. TYPOGRAPHY — sizes, weights, line-heights
5. SHAPES — radius, shadows, decorative elements
6. IMAGES/ICONS — size, position, aspect ratio

Be HARSH. Rate each section: MATCH / CLOSE / WRONG.
Return JSON: { "overall": "...", "sections": [...] }
```

**Tier 2: Figma fidelity check**

Same subagent pattern, but compare POST-CMS against the Figma reference:
```
FIGMA: .treble/figma/{slug}/reference.png
IMPL: .treble/screenshots/{PageName}-cms.png
```

**Fix loop:** Max 3 attempts per section. If a fix changes styling, the slice wrapper is wrong — fix the data mapping, not the component.

Write results to `build-state.json`:
```json
{
  "HeroSection": {
    "cmsStatus": "done",
    "cmsProvider": "prismic",
    "slicePath": "src/slices/HeroSection/",
    "visualReview": { "passed": true, "attempts": 1 }
  }
}
```

### 8. Advance

Once visual review passes:
1. Update `build-state.json`
2. Commit: `git add src/slices/{SliceName}/ customtypes/ .treble/build-state.json && git commit -m "cms(prismic): wire {SliceName} slice"`
3. Move to the next section
4. Go back to step 1

## After All Sections

1. Push slice models to Prismic: tell the user to run `npm run slicemachine` and click "Push" in the Slice Machine UI
2. Create seed content: tell the user to create a "home" page document in Prismic dashboard and populate each slice with the hardcoded content from the original build
3. Set up the revalidation webhook (in Prismic dashboard → Webhooks → point to `/api/revalidate`)
4. Verify preview mode works: navigate to a page, open Prismic toolbar, edit a field, see it update

## Stopping

- Stop after all sections are wired
- Stop if the user says stop
- Stop after 3 failed visual review attempts on a single section (mark as `"skipped"`)

## Summary

After finishing, tell the user:
- How many sections wired vs planned vs skipped
- Manual steps remaining (push models, create content, set up webhook)
- Any sections that failed visual review
