# OpenDav v0.4.0 - Multi-Session Telemetry & Playback

We are thrilled to announce the release of **OpenDav v0.4.0**, an action-packed update that drastically increases your telemetry analysis capability by introducing **Multi-Session Comparisons** and **Interactive Playback** functionality!

## Key Features

### Multi-File Telemetry Comparison
You can now load multiple telemetry files into OpenDav simultaneously! This enables side-by-side analysis of different sessions or driver comparisons. 
- **Session Selector UI**: A new, intuitive multi-file sidebar interface allows you to easily switch between loaded telemetry files.
- **Reference Laps Across Sessions**: The C (Cyan) and W (White) reference laps can now be selected from *any* loaded session and will instantly overlay onto the active session's traces on the graphing workspace.
- **Visual Segregation**: The sidebar beautifully separates each loaded file into filled header blocks, with hollow list boxes displaying the available laps within each session.

### Interactive Lap Playback
Analyzing where you lose or gain time is now easier than ever with a real-time lap playback feature.
- **Live Playback Controls**: Dedicated Play/Pause, Rewind, and Speed controls have been seamlessly integrated at the top-left of the graphing interface, directly above the tooltips.
- **Synchronized Track Map Cursor**: The active car's position is animated across the interactive track map alongside your cursor scrubbing over the telemetry traces. Watch the telemetry unfold in real-time or slow motion (up to 4x speed adjustment)!

## Improvements & Refactoring
- **Global State Overhaul**: `OpenDavApp` has been comprehensively refactored to support a `sessions: Vec<LoadedSession>` array-based state structure instead of being limited to a single dataset.
- **Plot Modernization**: Overlay rendering logic dynamically fetches cross-session cache slices to draw disparate lap data on the exact same graph cleanly.
- **UI Code Cleanup**: Deprecated `egui` functions and rounding methods have been fully purged from the application, keeping the UI codebase healthy and compatible with the latest stable ecosystem.
- **Memory Safety Enhancements**: Squashed numerous Rust borrow-checker conflicts that originally bottlenecked concurrent rendering tasks in the UI loops.

*Get out on the track, compare your sessions, and find those extra tenths!*
