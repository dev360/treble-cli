---
description: Compare implementation screenshot against Figma reference
arguments:
  - name: component
    description: Component or page name to compare (e.g. "HeroSection", "Homepage")
    required: true
---

# /treble:compare — Visual Comparison

Compare a built component's rendered output against the Figma reference image. This does a REAL side-by-side comparison, not just a "does it render" check.

## Steps

### 1. Find the Figma reference

Look up the component in `.treble/analysis.json`:
- Find `referenceImages` paths — these are the Figma screenshots on disk
- If no referenceImages, render one: `treble show "{nodeId}" --frame "{frameName}" --json`

### 2. Screenshot the implementation (via chrome-devtools-tester subagent)

Spawn a `chrome-devtools-tester` subagent:

```
Navigate to the running dev server (check localhost:3000, 3001, 5173, or whatever port is configured).
Set viewport to 1440px width.
Wait for full page load (network idle).
Take a full-page screenshot and save to .treble/screenshots/{component}-impl.png

If the component is a section (not a full page), scroll to it and take a targeted screenshot.

Return the screenshot file path.
```

### 3. Run the diff in a Sonnet subagent — NEVER in this conversation

> **HARD RULE — read this before doing anything else:**
> Do NOT call the `Read` tool on any PNG path in this conversation. Do NOT open `referenceImages[*]` or `.treble/screenshots/*.png` yourself. PNG bytes are huge and blow out the orchestrator's context after 2–3 components, which kills the build loop. Every diff happens inside a subagent. The orchestrator only ever sees the JSON the subagent returns.

Invoke the `Agent` tool with these **exact** parameters:

- `subagent_type`: `"general-purpose"`
- `model`: `"sonnet"` — REQUIRED. Sonnet 4.6 is more than capable of visual diffing; using the orchestrator's default (Opus) wastes context budget on a vision task and is the single biggest cost driver for multi-component builds. Do not omit this field.
- `description`: `"Visual diff: {component}"`
- `prompt`: the diff prompt below

Diff prompt to pass as `prompt`:

```
You are a pixel-perfectionist UI reviewer. Use the Read tool to load BOTH images, then compare them.

FIGMA DESIGN: {figma reference path}
IMPLEMENTATION: .treble/screenshots/{component}-impl.png

Go section by section. For EACH area, check:
- LAYOUT: element positions, flex direction, grid structure, alignment
- SPACING: margins, padding, gaps between elements
- COLORS: backgrounds, text colors, borders, gradients
- TYPOGRAPHY: font size, weight, line height, letter spacing, family
- SHAPES: border radius, shadows, decorative elements
- CONTENT: is placeholder content roughly appropriate?

Be BRUTAL. Flag every difference no matter how small. This is about pixel perfection.

Rate each section: MATCH / CLOSE / WRONG.

Return ONLY this JSON object — no prose, no markdown fences, no preamble:
{
  "overall": "MATCH|CLOSE|WRONG",
  "sections": [
    {
      "name": "section name",
      "rating": "MATCH|CLOSE|WRONG",
      "discrepancies": ["specific issue"],
      "fix": "specific code change"
    }
  ],
  "summary": "one sentence overall assessment"
}
```

The subagent returns the JSON to you. You never see the pixels.

### 4. Report and fix

Show the user the JSON the subagent returned. If discrepancies found:
1. Fix the implementation code (text-only — work from the discrepancy descriptions, do NOT re-open the screenshots)
2. Re-run step 2 + 3 (max 2 fix-compare cycles)
3. Update `.treble/build-state.json` with the review result
