# OpenDav v0.4.1 - Telemetry Deltas & Alignment Fixes

Welcome to **OpenDav v0.4.1**! This is a small but mighty patch following our major v0.4.0 update, focusing primarily on fixing a telemetry alignment bug and adding a highly requested lap comparison feature directly into the graphing HUD.

## What's New

### Multi-Lap Reference Deltas
When comparing against a reference lap, the HUD tooltips will now seamlessly expand into 3 beautifully aligned rows exactly where your cursor is positioned:
- **Base Lap Values**: Your primary session's telemetry using the standard color-coded system.
- **Reference Lap Values**: The telemetry of the exact same point on the track for the reference lap, colored in Cyan or White to match your overlay selection.
- **Delta Values**: A direct difference readout (`Reference - Base`), giving you an immediate, precise view of how much more or less input you applied at any given point on the track!

## Bug Fixes

### Telemetry Tooltip Alignment
Fixed a bug where the HUD telemetry tooltips (Speed, RPM, Throttle, etc.) were misaligned and delayed relative to the plot cursor. This was caused by idle pit lane and off-track data being cleanly truncated from the visual plot caches, shifting the cache indices. The tooltips now query the raw dataframe with a perfect 1:1 index mapping, ensuring lightning-fast and perfectly synchronized data readouts!

---
*Happy analyzing, and keep pushing for those marginal gains!*
