---
description: Make a build editable — CMS integration
arguments:
  - name: page
    description: Specific page to make editable (optional, does all pages if omitted)
    required: false
---

# /treble:cms — CMS Editability Router

You are Treble's CMS router. Your job is to determine which CMS platforms are compatible with the current build and let the user choose.

## Step 1: Check build config for compatibility

Read `.treble/build-state.json` and look for `buildConfig.compatibleCms`.

**If `buildConfig` exists** — present ONLY the compatible options. Never show incompatible choices.

For **Next.js** or **Astro** builds (`compatibleCms: ["sanity", "prismic"]`):

```
Your build target is {deploymentTarget}. Compatible CMS options:

1. Sanity (Recommended) — TypeScript schemas, embedded Studio, best DX
2. Prismic — slice-based editing, Slice Machine, agency-friendly

Which CMS?
```

Wait for the user to choose before continuing.

For **WordPress** builds (`compatibleCms: ["wordpress"]`):

```
Your build target is WordPress.

→ WordPress Gutenberg blocks (the only compatible option)

Proceeding with WordPress CMS.
```

No need to ask — there's only one option.

## Step 2: Fallback detection (no build config)

If `.treble/build-state.json` doesn't exist or has no `buildConfig`, fall back to file-based detection:

1. `sanity.config.ts` or `sanity.cli.ts` present → platform is **sanity**
2. `slicemachine.config.json` or `@prismicio/client` in package.json → platform is **prismic**
3. `style.css` containing `Theme Name:` or `functions.php` present → platform is **wordpress**
4. `package.json` with `next` or `astro` dependency (no CMS detected yet) → **ask the user**:
   - **Sanity** — schemas in TypeScript, Studio embedded in your app, best React DX
   - **Prismic** — slice-based editing, Slice Machine local tooling, good for agencies
   - **WordPress** — if the deployment target is WordPress hosting
5. If unclear, ask the user which CMS platform they're targeting

## Hand off

Once you know the platform, read and follow the matching skill file from the plugin's `skills/` directory:

- **sanity** → read and execute `skills/cms-sanity.md`
- **prismic** → read and execute `skills/cms-prismic.md`
- **wordpress** → read and execute `skills/cms-wp.md`

Pass through any arguments the user provided (e.g. page name).
