# OpenDAV v0.9.0 Release Notes

Welcome to OpenDAV v0.9.0! This release introduces gorgeous, high-resolution satellite mapping directly into the core track map renderer, alongside major layout improvements to the Sector Reports tab and a flawless new aspect-ratio scaling algorithm.

## What's New

### Satellite Map Integration (Beta)
- **Native Mapbox Static Imagery:** We've integrated the Mapbox Static Images API directly into the Rust backend. With a simple toggle, you can now seamlessly render beautiful high-resolution satellite imagery natively underneath the live telemetry trace.
- **Smart Local Caching:** To save network bandwidth and API costs, OpenDAV now natively caches fetched satellite maps to your local disk. Once you load a track with the satellite map enabled, it will instantly load from your local cache on all future sessions!
- **Bring-Your-Own-Key (BYOK):** Added a dedicated section in the Settings tab to securely input and persist your own Mapbox API key.

### Layout & Rendering Upgrades
- **Sector Reports Overhaul:** We've completely redesigned the Sector Analysis tab. Instead of the old side-by-side square layout, we now feature a premium vertical layout: the lap times grid stretches across the full width at the top, while the track map takes up the remaining screen real estate as a massive, full-width rectangle underneath.
- **Perfect Aspect-Ratio Scaling:** Rewrote the track map bounds calculator to inject custom padding logic. Whether your track is short and wide like Long Beach, or tall and narrow like Road Atlanta, the track map will mathematically fit perfectly into any window shape without ever clipping the top or bottom of the track.
- **Instant Snap-to-Fit:** Added dynamic lifecycle triggers that instantly recalculate the bounding box the exact millisecond you switch between the Dashboard, Graphs, or Sector Reports. The track map will never look out of place or "blank" until you double-click.
- **Wrap-Around Checkboxes:** Overhauled the track map UI headers to utilize fluid horizontal wrapping, ensuring that track sector checkboxes gracefully flow to the next line on smaller monitors instead of breaking the layout.

## Improvements & Fixes
- **Double-Click Reset Fix:** Fixed a race condition where double-clicking the map to snap the camera bounds would get overwritten by the previous frame's layout cycle. Double-clicking now flawlessly disables auto-follow and snaps to the full track layout.
- **Removed Deprecated Python Scripts:** Stripped out a bunch of loose, unused Python patching scripts from the root directory to keep the repository immaculate.

---
*Happy Racing!*
- The OpenDAV Team
