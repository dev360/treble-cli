---
name: cms-sanity
description: Make a React + shadcn/ui build editable via Sanity Studio. Use this skill when /treble:cms determines the CMS platform is Sanity.
version: 0.1.0
---

# Sanity CMS Integration (React + shadcn/ui)

You are Treble's CMS Agent for Sanity. The dev agent has already built pixel-perfect pages using the feature-based architecture (`src/features/*/`). Your job is to make content editable via Sanity — without breaking visual fidelity.

**You are doing a refactor, not a rebuild.** Components already look correct. You are swapping hardcoded strings and images for Sanity document fields — nothing else changes.

**CRITICAL:** Do NOT change layout, styling, or visual structure. Every change is a content source swap — hardcoded value → GROQ query result.

## Why Sanity works well for agentic setup

- **Schemas are TypeScript** — the agent writes them directly, no separate tooling
- **Studio is a React app** — runs in the same project, same `npm run dev`
- **GROQ is simple** — one query language, no GraphQL complexity
- **No model push step** — schemas deploy with the code

## Prerequisites

- The dev build must be complete — pages implemented and visually reviewed
- `.treble/analysis.json` and `.treble/build-state.json` must exist
- Project must be Next.js (App Router) — Sanity's primary integration target

## Treble CLI Tools

Use these during visual verification to pull exact Figma measurements:

- `treble tree "{frameName}" --root "{nodeId}" --verbose` — node tree with visual properties
- `treble tree "{frameName}" --root "{nodeId}" --json` — machine-readable node data
- `treble show "{nodeId}" --frame "{frameName}" --json` — screenshot of a specific Figma node

**On-disk design data:**
- `.treble/figma/{slug}/reference.png` — full-frame Figma screenshot (ground truth)
- `.treble/figma/{slug}/nodes.json` — all Figma nodes with properties
- `.treble/analysis.json` — component inventory, design system, tokens

## Step 0: Sanity Setup

### 0a. Check for existing setup

If `sanity.config.ts` exists, skip to Step 1.

### 0b. Account check

Ask the user: **"Do you have a Sanity project? If not, create a free one at sanity.io/manage and tell me the project ID."**

The agent cannot create the Sanity project — this is the ONE manual step.

### 0c. Initialize Sanity in the project

```bash
npm install sanity @sanity/vision @sanity/image-url next-sanity
```

Create `sanity.config.ts` in the project root:

```ts
import { defineConfig } from "sanity"
import { structureTool } from "sanity/structure"
import { visionTool } from "@sanity/vision"
import { schemaTypes } from "./src/sanity/schemas"

export default defineConfig({
  name: "default",
  title: "Site Name",
  projectId: process.env.NEXT_PUBLIC_SANITY_PROJECT_ID!,
  dataset: process.env.NEXT_PUBLIC_SANITY_DATASET || "production",
  plugins: [structureTool(), visionTool()],
  schema: { types: schemaTypes },
})
```

### 0d. Create the Sanity client

Write `src/sanity/client.ts`:

```ts
import { createClient } from "next-sanity"

export const client = createClient({
  projectId: process.env.NEXT_PUBLIC_SANITY_PROJECT_ID!,
  dataset: process.env.NEXT_PUBLIC_SANITY_DATASET || "production",
  apiVersion: "2024-01-01",
  useCdn: process.env.NODE_ENV === "production",
})

// For draft/preview mode
export const previewClient = createClient({
  projectId: process.env.NEXT_PUBLIC_SANITY_PROJECT_ID!,
  dataset: process.env.NEXT_PUBLIC_SANITY_DATASET || "production",
  apiVersion: "2024-01-01",
  useCdn: false,
  token: process.env.SANITY_API_READ_TOKEN,
  perspective: "previewDrafts",
})

export function getClient(preview = false) {
  return preview ? previewClient : client
}
```

### 0e. Add env vars

```bash
# .env.local
NEXT_PUBLIC_SANITY_PROJECT_ID=your_project_id
NEXT_PUBLIC_SANITY_DATASET=production
SANITY_API_READ_TOKEN=your_read_token
```

### 0f. Embed Sanity Studio

Create `src/app/studio/[[...tool]]/page.tsx`:

```tsx
"use client"
import { NextStudio } from "next-sanity/studio"
import config from "../../../../sanity.config"

export default function StudioPage() {
  return <NextStudio config={config} />
}
```

Add to `next.config.ts`:
```ts
// Silence Sanity Studio webpack warnings
webpack: (config) => {
  config.resolve.alias = { ...config.resolve.alias, canvas: false }
  return config
}
```

Now `http://localhost:3000/studio` is the CMS editor — no separate deploy needed.

### 0g. Create schema barrel

Write `src/sanity/schemas/index.ts`:

```ts
// Import all schema types here — add to this as you wire sections
import { page } from "./page"

export const schemaTypes = [page]
```

Write `src/sanity/schemas/page.ts`:

```ts
import { defineType, defineField } from "sanity"

export const page = defineType({
  name: "page",
  title: "Page",
  type: "document",
  fields: [
    defineField({
      name: "title",
      title: "Page Title",
      type: "string",
      validation: (r) => r.required(),
    }),
    defineField({
      name: "slug",
      title: "URL Slug",
      type: "slug",
      options: { source: "title", maxLength: 96 },
      validation: (r) => r.required(),
    }),
    defineField({
      name: "sections",
      title: "Page Sections",
      type: "array",
      of: [], // Populated as sections are wired
    }),
  ],
})
```

Commit: `git commit -m "chore: initialize Sanity Studio"`

---

## The Loop

For each section component in the build order:

### 1. Pick the next component

Read `.treble/build-state.json`. Find the next organism/section where `cmsStatus` is not `"done"`. Skip atoms — they receive data via props from their parent section.

### 2. Analyze what's editable

Read the component's source code. Identify every hardcoded value:

- **Text strings** → `string` (short) or `text` (long) or `blockContent` (rich text)
- **Images** → `image` type
- **Links/URLs** → `url` type or `object` with `{ label: string, url: url }`
- **Repeating items** (cards, features) → `array` of `object`
- **Boolean toggles** → `boolean`
- **Select options** → `string` with `options.list`

### 3. Create the schema

Write `src/sanity/schemas/{sectionName}.ts`:

```ts
import { defineType, defineField } from "sanity"

export const heroSection = defineType({
  name: "heroSection",
  title: "Hero Section",
  type: "object", // "object" for sections embedded in page, "document" for standalone
  fields: [
    defineField({
      name: "heading",
      title: "Heading",
      type: "string",
      validation: (r) => r.required(),
    }),
    defineField({
      name: "subheading",
      title: "Subheading",
      type: "text",
      rows: 3,
    }),
    defineField({
      name: "ctaLabel",
      title: "CTA Button Label",
      type: "string",
    }),
    defineField({
      name: "ctaUrl",
      title: "CTA Button URL",
      type: "url",
    }),
    defineField({
      name: "backgroundImage",
      title: "Background Image",
      type: "image",
      options: { hotspot: true }, // enables focal point cropping
    }),
  ],
  // Preview in the Studio list
  preview: {
    select: { title: "heading", media: "backgroundImage" },
  },
})
```

**Field type reference (Sanity):**

| Sanity type | Use for |
|------------|---------|
| `string` | Short text (headings, labels, names) |
| `text` | Long text (descriptions, bios) — add `rows: 3` for multiline |
| `blockContent` | Rich text — define as a custom type with `defineArrayMember` |
| `image` | Image with optional hotspot — use `options: { hotspot: true }` |
| `url` | Full URL string |
| `slug` | URL slug — auto-generates from another field |
| `boolean` | Toggle |
| `number` | Numeric value |
| `date` / `datetime` | Date fields |
| `reference` | Link to another document — `to: [{ type: "author" }]` |
| `array` of `object` | Repeating items (cards, features, list items) |

**For rich text (blockContent):**

```ts
export const blockContent = defineType({
  name: "blockContent",
  title: "Block Content",
  type: "array",
  of: [
    { type: "block" }, // paragraph, headings, lists, marks
    {
      type: "image",
      options: { hotspot: true },
    },
  ],
})
```

**For repeating items:**

```ts
defineField({
  name: "features",
  title: "Features",
  type: "array",
  of: [
    {
      type: "object",
      fields: [
        defineField({ name: "title", type: "string" }),
        defineField({ name: "description", type: "text" }),
        defineField({ name: "icon", type: "string", description: "Lucide icon name" }),
      ],
      preview: { select: { title: "title" } },
    },
  ],
})
```

### 4. Register the schema

Add the import to `src/sanity/schemas/index.ts`:

```ts
import { page } from "./page"
import { heroSection } from "./heroSection"
import { featureGrid } from "./featureGrid"

export const schemaTypes = [page, heroSection, featureGrid]
```

Add the section to the page schema's `sections` array:

```ts
defineField({
  name: "sections",
  title: "Page Sections",
  type: "array",
  of: [
    { type: "heroSection" },
    { type: "featureGrid" },
    // Add each section as you wire it
  ],
})
```

### 5. Write the GROQ query + data fetcher

Write `src/sanity/queries/{pageName}.ts`:

```ts
import { groq } from "next-sanity"
import { getClient } from "../client"

export const homePageQuery = groq`
  *[_type == "page" && slug.current == "home"][0] {
    title,
    sections[] {
      _type,
      _key,
      // Hero fields
      _type == "heroSection" => {
        heading,
        subheading,
        ctaLabel,
        ctaUrl,
        "backgroundImage": backgroundImage.asset->url
      },
      // Feature grid fields
      _type == "featureGrid" => {
        heading,
        features[] {
          title,
          description,
          icon
        }
      }
    }
  }
`

export async function fetchHomePage(preview = false) {
  return getClient(preview).fetch(homePageQuery)
}
```

**GROQ patterns:**
- `*[_type == "page"]` — all documents of a type
- `*[_type == "page" && slug.current == $slug][0]` — single by slug (parameterized)
- `backgroundImage.asset->url` — resolve image reference to CDN URL
- `sections[] { _type, _key, ... }` — expand array items
- `_type == "heroSection" => { ... }` — conditional projection per section type

### 6. Update the page to fetch from Sanity

Replace the feature page's hardcoded composition with Sanity data:

**Before** (`src/app/page.tsx`):
```tsx
import { HomePage } from "@/features/home/home-page"
export default function Page() { return <HomePage /> }
```

**After** (`src/app/page.tsx`):
```tsx
import { draftMode } from "next/headers"
import { fetchHomePage } from "@/sanity/queries/home"
import { SectionRenderer } from "@/sanity/section-renderer"

export default async function Page() {
  const { isEnabled } = await draftMode()
  const page = await fetchHomePage(isEnabled)
  return <SectionRenderer sections={page.sections} />
}
```

Write `src/sanity/section-renderer.tsx`:

```tsx
import { HeroSection } from "@/features/home/components/HeroSection"
import { FeatureGrid } from "@/features/home/components/FeatureGrid"

const sectionMap: Record<string, React.ComponentType<any>> = {
  heroSection: HeroSection,
  featureGrid: FeatureGrid,
}

interface SectionRendererProps {
  sections: Array<{ _type: string; _key: string; [key: string]: unknown }>
}

export function SectionRenderer({ sections }: SectionRendererProps) {
  if (!sections) return null
  return (
    <>
      {sections.map((section) => {
        const Component = sectionMap[section._type]
        if (!Component) return null
        return <Component key={section._key} {...section} />
      })}
    </>
  )
}
```

**Key pattern:** The section renderer maps `_type` strings to existing feature components. Components receive the GROQ query result as props. The GROQ query is shaped to match the component's prop interface — adapt the query, not the component.

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

**Fix loop:** Max 3 attempts per section. If a fix changes styling, the problem is the data mapping — fix the GROQ query shape or the section-renderer prop mapping, NOT the component itself.

Write results to `build-state.json`:
```json
{
  "HeroSection": {
    "cmsStatus": "done",
    "cmsProvider": "sanity",
    "schemaPath": "src/sanity/schemas/heroSection.ts",
    "visualReview": { "passed": true, "attempts": 1 }
  }
}
```

### 8. Advance

Once visual review passes:
1. Update `build-state.json`
2. Commit: `git add src/sanity/schemas/ src/sanity/queries/ .treble/build-state.json && git commit -m "cms(sanity): wire {SectionName} schema + query"`
3. Move to the next section
4. Go back to step 1

## After All Sections

1. **Seed content:** Tell the user to open `http://localhost:3000/studio`, create a "Page" document with slug "home", and add each section with the original hardcoded content
2. **Image migration:** Copy images from `public/images/` → upload to Sanity via Studio (or use the Sanity CLI: `sanity dataset import`)
3. **Deploy Studio:** Studio is already embedded at `/studio` — it deploys with the Next.js app
4. **Set up revalidation:** Add a Sanity webhook (Settings → API → Webhooks) pointing to `/api/revalidate` for on-demand ISR
5. **Preview mode:** Set up draft mode route handlers for editors to preview unpublished changes

## Stopping

- Stop after all sections are wired
- Stop if the user says stop
- Stop after 3 failed visual review attempts on a single section (mark as `"skipped"`)

## Summary

After finishing, tell the user:
- How many sections wired vs planned vs skipped
- Manual steps remaining (seed content, upload images)
- The Studio URL (`/studio`) and how to use it
- Any sections that failed visual review
