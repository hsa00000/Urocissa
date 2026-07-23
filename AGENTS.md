# Urocissa Agent Instructions

This file applies to the entire repository. The frontend rules below are mandatory for
`gallery-frontend` and for any other Vue/Vuetify interface added to this repository.

## Vuetify-first frontend policy

All frontend work must preserve a consistent Vuetify visual language. Prefer Vuetify's
components, props, slots, theme system, display helpers, and utility classes over native
HTML layout wrappers or hand-written visual systems.

### 1. Use Vuetify components before native HTML

- Use the closest Vuetify component whenever one exists:
  - Layout: `VContainer`, `VRow`, `VCol`, `VSheet`, `VSpacer`.
  - Surfaces: `VCard`, `VCardItem`, `VCardText`, `VCardActions`, `VToolbar`.
  - Lists and grouped settings: `VList`, `VListItem`, `VListSubheader`, `VDivider`.
  - Controls: `VBtn`, `VCheckbox`, `VCheckboxBtn`, `VSwitch`, `VTextField`,
    `VSelect`, `VAutocomplete`, `VCombobox`, and other Vuetify inputs.
  - Feedback: `VAlert`, `VProgressLinear`, `VProgressCircular`, `VEmptyState`,
    `VSnackbar`, `VTooltip`.
  - Navigation and overlays: `VTabs`, `VWindow`, `VDialog`, `VMenu`,
    `VNavigationDrawer`.
- Do not use raw `div`, `span`, or `section` elements merely to create spacing,
  alignment, surfaces, rows, headers, or groups that Vuetify can express.
- Do not build buttons, inputs, selects, checkboxes, dialogs, menus, tabs, cards,
  chips, alerts, or progress indicators from native elements when a Vuetify component
  exists.
- Native semantic elements are allowed only when they add genuine document semantics
  or accessibility and there is no suitable Vuetify API. They must not be used as a
  shortcut for visual layout.
- Do not recreate an existing Vuetify component with custom CSS.

### 2. Use Vuetify props and slots as the primary styling API

- Prefer component props such as `color`, `variant`, `density`, `rounded`, `border`,
  `elevation`, `lines`, `slim`, `size`, `width`, and `max-width`.
- Prefer documented component slots such as `prepend`, `append`, `title`, `subtitle`,
  `actions`, and `activator` instead of custom wrapper markup.
- Prefer Vuetify spacing, flex, typography, sizing, and visibility utility classes.
- Add scoped CSS only when the required behavior cannot be expressed with Vuetify.
  Keep that CSS minimal and explain the exception in the implementation summary.
- Never use CSS to imitate a Vuetify prop or utility that already provides the same
  behavior.

### 3. Theme compatibility is mandatory

- Components must inherit the active Vuetify theme. Do not force `theme="dark"` or
  `theme="light"` unless the product requirement explicitly calls for an isolated theme.
- For dialogs, menus, snackbars, and other teleported overlays, audit the full shared
  wrapper chain as well as the feature component. A child dialog is not theme-compatible
  if a shared `VDialog`, `VCard`, `VToolbar`, or theme provider forces a theme or uses a
  fixed surface color.
- Fix unintended theme overrides at the highest shared wrapper that owns them. Do not
  compensate by passing the current theme into each child overlay.
- Use semantic Vuetify colors such as `primary`, `secondary`, `surface`, `error`,
  `warning`, `info`, and `success`.
- Prefer component color props and Vuetify theme variables.
- Do not hard-code visual colors with hex values, `rgb()`, `rgba()`, fixed white,
  fixed black, or light/dark-only border colors.
- Do not rely on color alone to communicate state. Pair it with text, an icon, or the
  component's semantic type.
- Every material UI change must be visually checked in both light and dark themes.
- Overlay theme verification must inspect the rendered overlay itself, not only the
  dimmed page behind it. Open the overlay once in each theme, then change the global
  theme while it remains open. Confirm the overlay root and primary surface switch to
  the matching `v-theme--light` / `v-theme--dark` class and that their computed
  background and foreground colors change accordingly.

### 4. Match existing Urocissa component patterns

- Before designing a component, inspect the closest existing modal, settings page,
  form, list, or toolbar in this repository.
- Reuse the same hierarchy, spacing scale, density, border treatment, rounding,
  header style, action placement, and scroll behavior where applicable.
- Related views must look like siblings. For example, tabs such as Create job and
  Job queue should share the same summary row, grouped-list structure, margins, and
  typography rather than introducing separate card or dashboard styles.
- Avoid one-off visual patterns when an established Urocissa pattern already exists.
- When existing components conflict with these rules, follow the newer
  Vuetify-first, theme-aware pattern and keep the change scoped to the requested work.

### 5. Distinguish device capability from viewport layout

- Use Vuetify display breakpoints only for responsive layout and available space.
- When interaction behavior depends on the physical input environment—touch long
  press, hover-only affordances, drag activation delay, or mobile gesture handling—use
  the existing `useConfigStore('mainId').isMobile` device flag. Do not infer the device
  from `mobile`, `smAndDown`, `mdAndUp`, viewport width, or drawer width.
- Desktop interaction must remain desktop interaction at every viewport width. In
  particular, narrowing a desktop window must not introduce touch-only long-press or
  gesture delays.
- For interaction code that branches on `configStore.isMobile`, test both boolean
  states and separately test a narrow desktop viewport so responsive layout changes
  cannot silently alter device behavior.

## Mandatory use of vuetify-mcp

The repository has `vuetify-mcp` available. Use it actively for Vuetify work rather
than relying on memory.

### Required workflow

1. Read the installed Vuetify version from `gallery-frontend/package.json` or the lock
   file.
2. For every non-trivial Vuetify component being introduced or substantially changed,
   query `vuetify-mcp` with `get_component_api_by_version` using that exact version.
3. Confirm relevant props, slots, events, defaults, and version-specific behavior
   before writing the template.
4. Use `get_feature_guide` for theme, accessibility, display/platform, icons, or other
   cross-component features when relevant.
5. For Vuetify 4 migrations or uncertain version behavior, consult
   `get_v4_breaking_changes`.
6. If the Vuetify MCP tools are deferred, discover them before proceeding.
7. If `vuetify-mcp` is unexpectedly unavailable, report that briefly and verify against
   the installed Vuetify source/types under `node_modules`; do not guess.

The MCP output is implementation guidance. The repository's installed package version,
TypeScript types, existing components, and rendered behavior remain the final authority.

## Vue component architecture

- Use Vue 3 Composition API with `<script setup lang="ts">`.
- Keep SFC sections ordered as `<script>`, `<template>`, then `<style>`.
- Keep source state minimal and derive presentation values with `computed`.
- Keep props read-only and use typed emits for upward communication.
- Split repeated or substantial presentation blocks into focused child components.
- Keep modal/page orchestration separate from reusable form, list, and row presentation.
- Do not introduce a composable for a pure, one-off formatting helper.

## Preferred Urocissa UI patterns

### Dialogs

- Prefer `VDialog` containing a theme-inheriting `VCard`.
- Use `VCardItem` or `VToolbar` for the title and close action.
- Use `VProgressLinear` for modal loading state.
- Use `VCardText` as the scrollable content area and `VCardActions` for actions.
- Fullscreen mobile dialogs must preserve proper scrolling and keep headers and tabs
  from shrinking.

### Forms and settings

- Prefer grouped `VList` sections with `border`, `rounded`, transparent backgrounds,
  `VListSubheader`, and `VDivider`.
- Use `VListItem` prepend/append slots for icons, controls, and actions.
- Keep titles and descriptions readable at narrow widths; use Vuetify `lines` and
  typography utilities rather than fixed heights.
- Destructive choices must use the `error` semantic color and include explicit warning
  text.

### Loading, empty, and error states

- Use `VProgressCircular` or `VProgressLinear` for loading.
- Use `VEmptyState` for an empty collection when appropriate.
- Use `VAlert` or theme-aware list rows for warnings and errors.
- Error messages and identifiers must wrap without creating horizontal overflow.

### Buttons and icons

- Use `VBtn` and `VIcon`.
- Icon-only controls require an accessible `aria-label`.
- Use MDI icons consistently with neighboring components.
- Preserve existing loading, disabled, keyboard, and click-propagation behavior.

## Required implementation and verification process

For any material Vue/Vuetify change:

1. Inspect the target component and its nearest visual peers.
2. State a short component-responsibility map for non-trivial work.
3. Query the relevant exact-version APIs through `vuetify-mcp`.
4. Implement with Vuetify components, props, slots, utilities, and semantic colors.
5. Preserve existing business logic, public props/emits, routes, API behavior, and
   accessibility unless the task explicitly changes them.
6. Run targeted TypeScript and ESLint checks while iterating.
7. Run the frontend validation appropriate to the change:
   - `npm run check`
   - `npm run test:unit`
   - `npm run lint`
   - `npm run build:only`
8. Visually verify material UI changes at minimum in:
   - Desktop light theme.
   - Desktop dark theme.
   - A 390px-wide mobile viewport.
   - A 390px-wide desktop-browser viewport for any device-dependent interaction; it
     must continue using desktop mouse and keyboard behavior when
     `configStore.isMobile` is false.
   - Relevant loading, empty, active, disabled, error, and destructive states.
   - For teleported overlays: opened after each theme is selected and left open during
     a live theme toggle. Screenshots or DOM measurements must include the overlay
     content itself.
9. Compare related screens side by side for hierarchy, spacing, density, border,
   typography, action placement, scrolling, and tab height.
10. Ensure there is no horizontal overflow, clipped labels, or theme-specific contrast
    regression.

## Final review checklist

Before considering frontend UI work complete, confirm:

- Could any raw layout or control element be replaced by a Vuetify component?
- Were exact-version component APIs checked with `vuetify-mcp`?
- Does the result follow the nearest Urocissa modal/settings/list pattern?
- Are all colors theme-aware and semantic?
- Does it work in both light and dark themes?
- Does it work at 390px without clipping or horizontal overflow?
- Are loading, empty, error, disabled, and destructive states understandable?
- Are icon-only actions accessible?
- Did TypeScript, tests, lint, and the production build pass?
- Were temporary previews, generated test entry points, and unrelated files left out of
  the final change?

## Scope and exceptions

- These rules do not authorize unrelated refactors.
- Preserve user changes in a dirty worktree.
- A user request may explicitly override a rule for a specific task.
- When a genuine exception requires native markup, custom CSS, or a non-Vuetify
  component, keep it narrowly scoped and explain why Vuetify could not satisfy the
  requirement.
