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

## Context Management

**NEVER read PNG/image files directly in the main conversation.** All image reading MUST happen inside subagents via the `Agent` tool. This prevents context window bloat that kills multi-component builds.

When you need to see a Figma reference or compare visuals, spawn a subagent to do the image work and return text results.

If you see "image dimension limit" errors, run `/compact` before continuing.

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

Read the component's analysis entry from `analysis.json` (TEXT — this is fine in main context):
- `tier` — determines complexity (atom = simple, organism = composed)
- `shadcnMatch` — if set, USE the shadcn component, don't rebuild it
- `composedOf` — import these (they should already be built)
- `figmaNodes` — which Figma layers this maps to
- `props`, `variants`, `tokens` — the component interface
- `filePath` — where to write the code
- `implementationNotes` — the detailed visual reproduction notes (THIS is your primary input)
- `referenceImages` — paths to screenshots (read these in a subagent, not here)

**Use a subagent to examine reference images.** Spawn an Agent that reads the referenceImages PNGs and returns a text description of what it sees — colors, layout, spacing, typography. This keeps images out of the main context.

Read node properties for exact measurements (TEXT — fine in main context):
```bash
treble tree "{frameName}" --root "{nodeId}" --verbose
treble tree "{frameName}" --root "{nodeId}" --json
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
- `image-extract` → check `extractedImages` in analysis.json first:
  - If `extractedImages` has entries, copy from `.treble/figma/{slug}/assets/{file}` → `public/images/`
  - Use `<img src="/images/{file}">` in the component code
  - If no extracted images exist, fall back to `treble show` to render a screenshot, or use placeholder colors

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
      "suggestions": ["Change text-3xl to text-5xl", "Add bg-[#CDB07A] to button"]
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

**SKIP visual review for atoms** (Button, Input, Badge) — they're too small to meaningfully screenshot. Only compare organisms and pages.

### 5. Architectural Review

After visual review passes, review the code architecturally (text-only, fine in main context):

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
