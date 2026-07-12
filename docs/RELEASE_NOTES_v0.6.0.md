# OpenDAV v0.6.0-rs Release Notes

Welcome to OpenDAV v0.6.0! This release introduces massive improvements to sector analysis, giving drivers deeper visual insights into their pace consistency, alongside critical quality-of-life workflow enhancements for managing multiple telemetry files.

## What's New

### 📈 Timing Graphs Tab
We've added a brand-new **Timing Graphs** worksheet to the Reports section! This gives you instant visual feedback on sector-by-sector performance across your entire stint.
- **Lap Consistency Line Charts:** View a dedicated line chart for every single sector/corner to instantly identify where you're losing or gaining time over a stint.
- **Direct Lap Number Overlays:** Every data point on the line charts is clearly labeled with its lap number, so you never lose track of which lap was an outlier.
- **Sector Anomaly Filtering:** Automatically filters out "out-laps" and crashed sectors via a dedicated toggle. This prevents massive time deltas from breaking the vertical scale of your charts, keeping your true pace in clear view.

### 🗂️ Advanced Multi-File Support
Working with multiple `.ibt` telemetry files is now completely seamless.
- **Primary File Dropdown:** A new top-bar dropdown menu appears whenever multiple files are loaded. You can now effortlessly swap the "Primary File" on the fly.
- **Dynamic Updates:** Changing the primary file instantly repopulates the Sector Reports, Track Map, and Dashboard metrics with the new session's data. 
- **Auto-Fastest Lap:** Switching primary files will now automatically snap the selected lap to the new session's fastest lap for immediate analysis.

### ✨ Polish & Fixes
- **Splash Screen Transition:** Fixed a visual glitch where a white `egui` loading spinner would flash briefly during the splash screen transition. The OpenDAV branding now fades in seamlessly!
- **UI Cleanups:** Removed redundant track layout venue texts to free up screen real-estate and improve readability around the track map.
- **Sector Reports Visuals:** Added sleek color-coded indicator boxes to the sector reports page for a cleaner, more polished look.
- **Graph Interaction Fixes:** Disabled accidental mouse-wheel zooming/scrolling on sector graphs so you can smoothly scroll down the reports page without causing visual scaling bugs.

---
*Happy Racing!*
- The OpenDAV Team
