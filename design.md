# Design — Universal Film Camera

A locked design system for this app. The interface is a professional imaging
instrument: the live image is primary, controls are quiet, and text appears only
when it carries shooting or asset-management information.

## Genre

Atmospheric, technical-austere.

## Macrostructure family

- App pages: Workbench. One dominant working surface with a compact icon rail.
- Camera: live image first; exposure values float over the image; shutter stays geometrically centred.
- Media and Settings: compact catalogue/workbench surfaces using the same rail, tokens, and control voice.
- Landscape Settings: master-detail. A readable category sidebar stays fixed while the selected panel scrolls independently.
- Landscape Media: catalogue split view. Status/collection filters occupy the narrow sidebar; thumbnails and details occupy the dominant pane.
- Portrait Settings and Media: compact icon tabs and stacked controls; never force the landscape split view into narrow widths.

## Theme

Use the existing dark neutral-cool OKLCH palette in `tokens.css`. Cyan-blue is
the sole navigation accent; red is reserved for recording/destructive state.

## Typography

- Display/body: Avenir Next or platform system fallback, upright.
- Technical values: SF Mono or platform monospace fallback.
- Labels are terse. Icon-only controls retain localized `aria-label` and `title` text.

## Spacing

Use the existing 4-point named scale in `tokens.css`. Safe Area is mandatory.

## Motion

Motion-cut. State colour and opacity transitions only; no decorative movement.

## Microinteractions stance

- Selected tools use the accent colour and a quiet elevated surface.
- Success is silent; errors remain explicit.
- Touch targets are at least 44pt, 48pt on coarse pointers.
- Icon-only controls expose names to assistive technology and native tooltips.

## Per-page allowances

- App pages do not use decorative enrichment; the camera image is the visual anchor.
- Text remains for exposure values, format choices, file identity, warnings, and destructive confirmation.
- Navigation, back, refresh, view switches, monitor tools, and settings tabs should be icon-first.
- In landscape master-detail pages, category labels may accompany icons because they communicate hierarchy rather than repeat an obvious action.

## What pages MUST share

- Dark canvas, cyan navigation accent, compact squared controls, mono technical values.
- Safe-area containment and responsive behavior at 320, 375, 414, and 768 CSS px.
- The same icon stroke, focus ring, touch target, and selected-state treatment.

## Responsive composition

- Portrait and widths below 40rem use one content column and a bottom tool rail.
- Landscape at 40rem and above uses a persistent right application rail.
- Settings uses a narrow left category pane and a wide right editor pane.
- Media uses a narrow left filter pane and a wide right catalogue pane; the catalogue owns vertical scrolling.
- Split panes use `minmax(0, 1fr)` for image-bearing tracks and remain inside the Safe Area.

## Exports

`tokens.css` at the project root is the runtime source of truth. The following
maps allow the same system to be reused without inventing a second palette.

### Tailwind v4 `@theme`

```css
@theme {
  --color-paper: oklch(12% 0.008 250);
  --color-paper-2: oklch(16% 0.01 250);
  --color-paper-3: oklch(21% 0.012 250);
  --color-ink: oklch(95% 0.008 250);
  --color-muted: oklch(78% 0.01 250);
  --color-accent: oklch(70% 0.16 235);
  --color-focus: oklch(82% 0.15 90);
  --font-display: "Avenir Next", "Segoe UI Variable Display", sans-serif;
  --font-body: "Avenir Next", "Segoe UI Variable Text", sans-serif;
  --font-mono: "SFMono-Regular", "Cascadia Mono", monospace;
  --spacing-xs: 0.5rem;
  --spacing-sm: 0.75rem;
  --spacing-md: 1rem;
  --spacing-lg: 1.5rem;
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
}
```

### DTCG `tokens.json`

```json
{
  "$schema": "https://design-tokens.github.io/community-group/format/",
  "color": {
    "paper": { "$value": "oklch(12% 0.008 250)", "$type": "color" },
    "paper-2": { "$value": "oklch(16% 0.01 250)", "$type": "color" },
    "paper-3": { "$value": "oklch(21% 0.012 250)", "$type": "color" },
    "ink": { "$value": "oklch(95% 0.008 250)", "$type": "color" },
    "accent": { "$value": "oklch(70% 0.16 235)", "$type": "color" },
    "focus": { "$value": "oklch(82% 0.15 90)", "$type": "color" }
  },
  "space": {
    "xs": { "$value": "0.5rem", "$type": "dimension" },
    "sm": { "$value": "0.75rem", "$type": "dimension" },
    "md": { "$value": "1rem", "$type": "dimension" },
    "lg": { "$value": "1.5rem", "$type": "dimension" }
  }
}
```

### shadcn/ui CSS variables

```css
:root {
  --background: 12% 0.008 250;
  --foreground: 95% 0.008 250;
  --card: 16% 0.01 250;
  --card-foreground: 95% 0.008 250;
  --popover: 16% 0.01 250;
  --popover-foreground: 95% 0.008 250;
  --primary: 70% 0.16 235;
  --primary-foreground: 13% 0.015 250;
  --secondary: 21% 0.012 250;
  --secondary-foreground: 95% 0.008 250;
  --muted: 34% 0.012 250;
  --muted-foreground: 78% 0.01 250;
  --border: 34% 0.012 250;
  --input: 34% 0.012 250;
  --ring: 82% 0.15 90;
  --radius: 0.25rem;
}
```
