---
name: Fluent Glass
colors:
  surface: '#f9f9f9'
  surface-dim: '#dadada'
  surface-bright: '#f9f9f9'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#f3f3f3'
  surface-container: '#eeeeee'
  surface-container-high: '#e8e8e8'
  surface-container-highest: '#e2e2e2'
  on-surface: '#1a1c1c'
  on-surface-variant: '#404752'
  inverse-surface: '#2f3131'
  inverse-on-surface: '#f1f1f1'
  outline: '#717783'
  outline-variant: '#c0c7d4'
  surface-tint: '#0060ab'
  primary: '#005faa'
  on-primary: '#ffffff'
  primary-container: '#0078d4'
  on-primary-container: '#ffffff'
  inverse-primary: '#a3c9ff'
  secondary: '#006876'
  on-secondary: '#ffffff'
  secondary-container: '#4de4fd'
  on-secondary-container: '#006370'
  tertiary: '#974700'
  on-tertiary: '#ffffff'
  tertiary-container: '#bc5b00'
  on-tertiary-container: '#ffffff'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  primary-fixed: '#d3e3ff'
  primary-fixed-dim: '#a3c9ff'
  on-primary-fixed: '#001c39'
  on-primary-fixed-variant: '#004883'
  secondary-fixed: '#a0efff'
  secondary-fixed-dim: '#3cd8f1'
  on-secondary-fixed: '#001f25'
  on-secondary-fixed-variant: '#004e59'
  tertiary-fixed: '#ffdbc8'
  tertiary-fixed-dim: '#ffb689'
  on-tertiary-fixed: '#311300'
  on-tertiary-fixed-variant: '#743500'
  background: '#f9f9f9'
  on-background: '#1a1c1c'
  surface-variant: '#e2e2e2'
typography:
  display:
    fontFamily: Plus Jakarta Sans
    fontSize: 40px
    fontWeight: '700'
    lineHeight: 48px
    letterSpacing: -0.02em
  headline-lg:
    fontFamily: Plus Jakarta Sans
    fontSize: 32px
    fontWeight: '600'
    lineHeight: 40px
    letterSpacing: -0.01em
  headline-md:
    fontFamily: Plus Jakarta Sans
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
  body-lg:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: '400'
    lineHeight: 28px
  body-md:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: '400'
    lineHeight: 24px
  body-sm:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  label-md:
    fontFamily: Geist
    fontSize: 12px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0.05em
  label-sm:
    fontFamily: Geist
    fontSize: 10px
    fontWeight: '600'
    lineHeight: 14px
    letterSpacing: 0.08em
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  unit: 4px
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 40px
  container-max: 1200px
  gutter: 24px
---

## Brand & Style
The design system is a modern interpretation of desktop operating system aesthetics, specifically tailored for a search-first productivity environment. It prioritizes clarity, depth, and a sense of "place" through the use of layered materials.

The style is **Glassmorphism** mixed with **Corporate Modernism**. It leverages semi-transparent surfaces to create a sense of hierarchy without heavy shadows. The emotional response should be one of calm efficiency—a focused workspace that feels lightweight and native to a high-end desktop environment. 

Key attributes:
- **Translucency:** Backgrounds are never fully opaque; they hint at the content behind them.
- **Precision:** Fine 1px borders define edges rather than heavy dropshadows.
- **Focus:** Interface elements recede to allow the user's content and search results to take center stage.

## Colors
The palette is intentionally restrained to maintain a professional, utility-driven atmosphere. 

- **Primary Blue:** A signature Windows-inspired blue (#0078D4) is used exclusively for primary actions, focus states, and active indicators.
- **Neutral Foundation:** We utilize a range of cool grays. The background is a soft light gray, while elevated surfaces use a semi-transparent white.
- **Glass Tint:** Surface colors are rarely solid. They are defined by an RGBA value that allows background colors to bleed through slightly, mimicking the "Mica" effect.

## Typography
The typography system uses a tiered approach to balance personality and utility. 

- **Headlines:** Plus Jakarta Sans provides a friendly, modern roundness that complements the soft corners of the UI.
- **Body:** Inter is used for all reading experiences to ensure maximum legibility and a systematic, neutral feel.
- **Labels/Code:** Geist is used for small labels, metadata, and any technical strings to provide a precise, developer-friendly touch.

For mobile, `display` and `headline-lg` should scale down by 20% to maintain comfortable line lengths.

## Layout & Spacing
This design system employs a **Fixed Grid** model for desktop, centered within the viewport to maintain focus.

- **The Search-First Layout:** The primary interaction point is a centered search bar. Spacing around this element is generous (`xl`) to reduce visual noise.
- **Grid:** A 12-column grid is used for dashboard layouts, while search results typically occupy a 10-column centered "reading lane" (approx. 800px) to prevent eye strain.
- **Rhythm:** All margins and paddings are multiples of 4px. Use `md` (16px) for standard internal component padding and `lg` (24px) for spacing between distinct sections.

## Elevation & Depth
Depth is achieved through material density rather than traditional shadows.

- **Level 0 (Base):** The application background. A solid or very subtle gradient.
- **Level 1 (Mica/Glass):** The primary surface for cards and search containers. Uses `rgba(255, 255, 255, 0.7)` with a `backdrop-filter: blur(20px)`.
- **Level 2 (Overlay):** Used for tooltips and dropdown menus. Uses a slightly higher opacity `rgba(255, 255, 255, 0.9)` and a subtle 1px border (`rgba(0, 0, 0, 0.1)`).
- **Edge Treatment:** Instead of shadows, use a "High-Light" border—a 1px solid stroke on the top and left edges that is slightly lighter than the surface, and a darker stroke on the bottom and right.

## Shapes
The shape language follows a consistent "Softened Desktop" approach. 

- **Standard Elements:** Buttons, inputs, and small cards use the `rounded` (8px) setting.
- **Large Containers:** Main search modals or layout sections use `rounded-lg` (16px).
- **Interactive States:** Hovering over a list item should reveal a `rounded-sm` (4px) background highlight.
- **Visual Continuity:** Every element must have a border radius; sharp corners are strictly avoided to maintain the approachable, modern aesthetic.

## Components
Consistent component styling ensures the system feels like a singular cohesive environment.

- **Search Input:** The hero component. Large, centered, with a glass background. On focus, the border transitions from neutral to Primary Blue with a subtle outer glow (2px blue at 20% opacity).
- **Buttons:** 
    - *Primary:* Solid Primary Blue, white text, 8px radius.
    - *Secondary:* Glass background, 1px neutral border, subtle hover lift.
- **Chips/Badges:** Small, `rounded-xl` (pill-shaped) with a very low-opacity gray background. Used for categories or filters.
- **Cards:** Semi-transparent "Mica" containers with 16px padding. Content is separated by thin horizontal rules (`rgba(0,0,0,0.05)`).
- **Lists:** Clean rows with no borders between items. Instead, use an 8px margin and a subtle gray background highlight on hover to indicate interactivity.
- **Checkboxes/Radios:** These use the Primary Blue for checked states. The checkmark should be a crisp, 2px stroke.