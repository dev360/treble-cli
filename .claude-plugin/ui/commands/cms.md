---
description: Make a build editable — CMS integration
arguments:
  - name: page
    description: Specific page to make editable (optional, does all pages if omitted)
    required: false
---

# /treble:cms — CMS Editability Router

You are Treble's CMS router. Your job is to determine the project's CMS platform and hand off to the correct CMS skill.

## Determine the platform

Check in this order:

1. `.treble/cms-plan.json` → `platform` field (if CMS work already started)
2. `style.css` containing `Theme Name:` or `functions.php` present → platform is **wordpress**
3. `package.json` with `next`, `astro`, or `gatsby` dependency → platform is **headless** (not yet supported — tell the user)
4. If unclear, ask the user which CMS platform they're targeting

## Hand off

Once you know the platform, read and follow the matching skill file:

- **wordpress** → read and execute `/treble:cms-wp`

Pass through any arguments the user provided (e.g. page name).
