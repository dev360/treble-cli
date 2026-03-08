---
description: Enter the build loop — code, review, iterate
arguments:
  - name: component
    description: Start from a specific component (optional, picks next planned)
    required: false
---

# /treble:dev — Build Loop

You are Treble's build router. Your job is to triage the design, choose the right deployment target, set up a solid project foundation, and hand off to the correct build skill.

## Step 0: Triage & Project Setup (FIRST PRIORITY)

Before writing any components, classify the design, pick a deployment target, and ensure the project is a well-organized, **runnable** repository.

If `package.json` already exists, the dev server starts, AND `.treble/build-state.json` has a `buildConfig` section, skip to "Hand off".

### 0a. Classify the design

Read `.treble/analysis.json` and classify by the section/component signals present:

| Signals in analysis.json | Classification |
|--------------------------|---------------|
| Hero, testimonials, feature grids, CTA buttons, pricing cards | **marketing-website** |
| Sidebar nav, data tables, forms, modals, tabs, breadcrumbs, user avatars | **web-app** |
| Product cards, cart, checkout flows, pricing tables | **ecommerce** |
| Article layout, author cards, tag lists, pagination | **blog** |
| Gallery grids, case studies, project cards | **portfolio** |

Look at sections, component names, and page structure. If multiple categories fit, pick the dominant one.

Tell the user what you found:

> This looks like a **multi-page marketing website** with 5 pages.

or

> This looks like a **web application** with dashboard, settings, and user management views.

### 0b. Present ranked deployment targets

Based on the classification, present ranked options. **Always ask — never auto-select.**

**Exclusion rules:**
- **Exclude WordPress** if classification is `web-app` or `ecommerce` (no WP for SaaS UIs or custom storefronts)
- **Rank Astro last** for `web-app` (shared state and auth are harder with islands architecture)
- **Always include Next.js** — it works for everything

For a **marketing-website**, **blog**, or **portfolio**:
```
Deployment options (ranked by fit):

1. Next.js (Recommended) — SSR/SSG, best ecosystem, works with any CMS later
2. Astro — static-first, faster for pure content sites
3. WordPress — if you need WP hosting or existing WP infrastructure
```

For a **web-app**:
```
Deployment options (ranked by fit):

1. Next.js (Recommended) — SSR/SSG, API routes, auth, shared state — built for this
2. Astro — possible with React islands, but shared state and auth are harder than in Next.js

(WordPress is not appropriate for this type of design.)
```

For **ecommerce**:
```
Deployment options (ranked by fit):

1. Next.js (Recommended) — SSR/SSG, API routes for cart/checkout, best ecosystem
2. Astro — static catalog pages with islands for interactive cart

(WordPress is not appropriate for custom e-commerce UIs.)
```

Wait for the user to choose before continuing.

### 0c. Ask where to place files (ALWAYS)

After the deployment target is chosen, ask where to set up the project:

```
Where should I set up the project?

Suggested: ./ (current directory)
Or specify a path:
```

If the current directory already has a `package.json`, note it and offer:
1. Build inside this existing project
2. Create a new subdirectory (suggest a name based on the design)

Wait for the user to confirm before continuing.

### 0d. Record build config

Write the triage decisions to `.treble/build-state.json` under a `buildConfig` key:

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

**Compatibility matrix:**

| Deployment Target | Compatible CMS | Build Skill |
|-------------------|---------------|-------------|
| Next.js | sanity, prismic | dev-shadcn |
| Astro | sanity, prismic | dev-shadcn |
| WordPress | wordpress | dev-basecoat-wp |

### 0e. Scaffold or verify the project

If starting fresh (no `package.json`):

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

**WordPress:** existing theme root is fine, skip scaffold.

**Verify it runs** — `npm run dev` must start without errors. Fix any issues before moving on.

If `package.json` exists, verify: `npm install && npm run dev` works. If it doesn't, fix it first.

### 0f. Project structure

Set up the feature-based directory structure (see `skills/dev-shadcn.md` for full rules):

```
src/
├── components/
│   ├── ui/              # shadcn primitives ONLY (managed by shadcn CLI)
│   ├── common/          # truly reusable across 2+ features (Logo, SocialLinks)
│   └── layout/          # page shells (Header, Footer, PageLayout, SectionContainer)
├── features/
│   └── [feature-name]/  # one per page/domain area
│       ├── components/  # feature-specific components
│       └── feature.tsx  # main export — mounted in pages/routes
├── lib/                 # utilities, helpers, cn()
└── app/ or pages/       # thin route files that mount features
public/
├── images/              # extracted Figma images
└── fonts/               # local font files (if any)
```

**Rule:** If you're about to write a file to `src/components/`, stop and ask: "Is this used by 2+ features?" If not, it belongs in `src/features/{name}/components/`.

### 0g. Testing setup (if appropriate)

Add a basic test runner. Skip for simple landing pages — add for apps with logic, forms, or interactivity.

```bash
npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom
```

Add to `vite.config.ts` (or `vitest.config.ts` for Next.js/Astro):
```ts
test: {
  environment: 'jsdom',
  setupFiles: './src/test/setup.ts',
}
```

Create `src/test/setup.ts`:
```ts
import '@testing-library/jest-dom'
```

Add `"test": "vitest"` to `package.json` scripts. Run `npm test` to verify.

### 0h. Database / backend services (if needed)

If the project needs a database (CMS, auth, etc.), use Docker so the repo is self-contained:

```yaml
# docker-compose.yml
services:
  db:
    image: postgres:16-alpine
    ports: ["5432:5432"]
    environment:
      POSTGRES_DB: app
      POSTGRES_USER: app
      POSTGRES_PASSWORD: app
    volumes:
      - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
```

Add to `package.json` scripts: `"db:up": "docker compose up -d"`, `"db:down": "docker compose down"`.

For simpler needs (Payload CMS, small apps), prefer **SQLite** — no Docker required.

### 0i. Git hygiene

```bash
git init  # if not already a repo
```

Ensure `.gitignore` covers: `node_modules/`, `dist/`, `.env.local`, `.treble-tmp/`, `.next/` (Next.js), `.astro/` (Astro).

**Commit the scaffold:** `git add -A && git commit -m "chore: initial project setup"`

This is your clean baseline. Every component build after this gets its own commit.

---

## Hand off

Once the project is set up and runnable, hand off to the correct build skill from the plugin's `skills/` directory:

- **shadcn** (Next.js or Astro) → read and execute `skills/dev-shadcn.md`
- **wordpress** → read and execute `skills/dev-basecoat-wp.md`

Pass through any arguments the user provided (e.g. component name).
