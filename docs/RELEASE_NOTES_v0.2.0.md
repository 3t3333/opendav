# OpenDav Release Notes — v0.2.0

**Release Date:** June 22, 2026

We are excited to announce the release of **OpenDav v0.2.0**! This release brings a major new visualization layout to the telemetry workspace, a premium redesign of the branding launch sequence, and critical fixes for input handling and data parser alignment.

---

## New Features

### 1. Workspace Split Layout (The "T" View)
You can now display a live, interactive track map directly below your active telemetry charts inside the graphs page.
- **Context-Aware Toggle ("T"):** Added a tiny uppercase **"T"** button to the top-right header (right next to the theme preference switcher). It is context-aware and only appears when viewing the Graphs page. When active, it turns orange (`ACCENT_COLOR`).
- **50/50 Screen Split:** Toggling this option divides the workspace height in half. The top half renders the charts, and the bottom half displays the interactive track map.
- **Interactive Scrubber Synchronization:** The track map is fully interactive (supports panning, zooming, and turn labels) and draws the live position dot showing exactly where the car is on track as you scrub the cursor across the line graphs.

---

## Architecture & Code Quality

### 2. Decoupled WorksheetConfig Specification
- **Clean Separation of Concerns:** Moved worksheet-specific layout metadata (channels, line colors, units, visual bounds) out of the core rendering loop and into a new `WorksheetConfig` struct system.
- **Abstract Specifications:** The `draw_motec_plot` graphing engine now only handles rendering and is completely agnostic of worksheet type. It translates abstract layout definitions (`WorksheetConfig::basic()` and `WorksheetConfig::rake()`) into lines and graphs dynamically.
- **Extensible Design:** Makes implementing additional charts, custom channels, or math worksheets in future releases extremely simple and clean.

---

## Design Updates

### 2. Premium Launch Splash Screen Redesign
The splash screen has been completely overhauled to align with OpenDav's sleek, high-end visual design:
- **Centered Transparent Branding:** Replaced the full-screen stretched image with the official `transparent_full_opendav_logo.png` centered on the screen, locked at a sharp `550px` width.
- **Obsidian Backdrop:** Painted the background a deep obsidian color (`#0A0A0A`), eliminating white-flash transitions and blending smoothly into the dashboard.

### 3. Track Map Corner Logo
- **Branding Header:** Switched the left sidebar navigation header from the old `header.png` to the new `corner_logo.png`.
- **Track Map Backdrop:** The new corner logo features a background design styled with snippets of track maps from Sebring and Snetterton, matching the electric orange theme.
- **Version Stamp:** The logo integrates the software release version label (`v0.2.0`) directly into the header design.

---

## Bug Fixes & Stability

### 3. Isolated Plot Interactions (No Crosstalk)
- **Problem:** Dragging/panning or scrolling the mouse wheel on the track map would leak inputs and pan/zoom the line graphs above.
- **Fix:** Swapped global event queries for isolated `egui::Response` checks (`clicked()`, `dragged()`, `drag_started()`, and `hovered()`). Dragging, zooming, or scrubbing on one element now stays completely isolated from the other.

### 4. Layout Zoom Persistence
- **Problem:** Toggling the track map or switching worksheet tabs caused the line graphs to zoom out completely, losing the user's focus window.
- **Fix:** Added change-tracking states (`previous_page` and `previous_show_graphs_track_map`) to trigger a shared viewport bounds sync precisely on the transition frames, preserving your exact zoom level.

### 5. Telemetry Header Offsets (Fixed in `generate_mock_ibt`)
- **Problem:** A 4-byte padding offset error shifted variable headers in mock data, causing telemetry columns to read as empty null data (`\0\0\0\0Lap`).
- **Fix:** corrected header padding offset math to `52 - 40` to perfectly align columns for the parser.

---

## What's Next
In future releases, we plan to implement driver coaching features, including speed heatmaps overlaid on the track maps and reference delta overlays.

*OpenDav Team*
