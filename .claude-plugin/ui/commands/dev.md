---
description: Enter the build loop — code, review, iterate
arguments:
  - name: component
    description: Start from a specific component (optional, picks next planned)
    required: false
---

# /treble:dev — Build Loop

You are Treble's Build Agent. Your job is to implement components from `.treble/analysis.json`, following a strict code → visual review → architectural review loop.

**CRITICAL:** ONLY use the `treble` CLI and local `.treble/` files for Figma data. Do NOT call the Figma API directly or use any Figma MCP server. All design data is on disk.

## Prerequisites

- `.treble/analysis.json` must exist (run `/treble:plan` first)
- `.treble/build-state.json` must exist
- The project should have a package.json and dev server configured

## The Loop

For each component in the build order:

### 1. Pick the next component

Read `.treble/build-state.json` and `.treble/analysis.json`. Find the next component where status is `"planned"`, following the `buildOrder` array.

If the user specified a component name, start there instead.

### 2. Gather context

Read the component's analysis entry from `analysis.json`:
- `tier` — determines complexity (atom = simple, organism = composed)
- `shadcnMatch` — if set, USE the shadcn component, don't rebuild it
- `composedOf` — import these (they should already be built)
- `figmaNodes` — which Figma layers this maps to
- `props`, `variants`, `tokens` — the component interface
- `filePath` — where to write the code

Look at the Figma reference:
```
Read .treble/figma/{frame-slug}/reference.png
```

For precise detail on this specific component, render it:
```bash
treble show "{nodeName}" --frame "{frameName}"
```
Then read the snapshot from `.treble/figma/{frame-slug}/snapshots/`.

Read node properties for exact measurements:
```bash
treble tree "{frameName}" --verbose
```

### 3. Code

Write the component following these rules:

**Atoms:**
- Use shadcn/ui if `shadcnMatch` is set — wrap/extend the shadcn component
- Generic props — no hardcoded content
- Design tokens from the analysis, mapped to Tailwind classes
- File at `src/components/{ComponentName}.tsx`

**Organisms (sections):**
- Import their `composedOf` dependencies
- Layout matching the Figma structure (flexbox, grid)
- Accept content via props — sections are layout containers
- File at `src/components/{ComponentName}.tsx`

**Pages:**
- Import all sections in order
- Pass concrete content to sections
- File at `src/pages/{PageName}.tsx`

**Assets:**
- `svg-extract` → render via `treble show`, extract SVG, save to `src/components/icons/`
- `icon-library` → import from lucide-react (or the matched library)
- `image-extract` → render via `treble show`, save to `public/images/`

### 4. Visual Review

After writing the code, review it visually:

1. Read the Figma reference image for this component's frame
2. If the dev server is running, take a screenshot of your implementation
3. Compare the two — check:
   - **Layout**: positions, flex direction, grid structure
   - **Spacing**: margins, padding, gaps
   - **Colors**: background, text, border colors vs design tokens
   - **Typography**: font size, weight, line height
   - **Border radius**: matches token values
   - **Shadows**: correct values applied

4. Write the visual review result to `build-state.json`:
```json
{
  "ComponentName": {
    "status": "implemented",
    "filePath": "src/components/ComponentName.tsx",
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

**If visual review fails** → go back to step 3, fix the code, increment `attempts`. Max 3 attempts before marking as `"skipped"`.

### 5. Architectural Review

After visual review passes, review the code architecturally:

1. Is it using shadcn correctly? Not re-implementing what shadcn provides?
2. Are props generic? No hardcoded strings that should be props?
3. Is the component properly composed? Using its `composedOf` dependencies?
4. Is it following React/Tailwind conventions?
5. Is the Tailwind usage correct? Using design tokens, not arbitrary values?
6. Is the component properly typed (TypeScript)?

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
2. Commit: `git add src/components/{ComponentName}.tsx .treble/build-state.json && git commit -m "feat: implement {ComponentName}"`
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
