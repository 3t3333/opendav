# OpenDAV v0.1.0
<img width="1200" height="400" alt="Untitled - June 18, 2026 at 23 59 55-1" src="https://github.com/user-attachments/assets/bb317e43-8f47-4222-88eb-d2955ffb2765" />

OpenDav is an open, performance centered telemetry analysis workspace built natively in Rust. This release lays the foundation for high-performance telemetry ingestion, interactive graphing, dynamic spatial visualization, and sector analysis.



## Technical Highlights & Core Features

### 1. GPS-Derived 2D Vector Track Map
* **Coordinate Projection**: Automatically projects raw spherical latitude and longitude telemetry points to a local Cartesian plane in meters relative to the lap start coordinate:

$$
x_i = R \cdot (\text{Lon}_i - \text{Lon}_0) \cdot \frac{\pi}{180} \cdot \cos\left(\text{Lat}_0 \cdot \frac{\pi}{180}\right)
$$

$$
y_i = R \cdot (\text{Lat}_i - \text{Lat}_0) \cdot \frac{\pi}{180}
$$
* **Dynamic Segments**: Renders the active driven lap path outline as a bold line (`3.0` width).
* **Coordinate Jump Handling**: Automatically splits rendering paths if the spatial distance between consecutive points exceeds 50 meters (e.g., driver resets to the garage or pit stalls). This eliminates straight lines cutting diagonally across the track map.
* **Reference Anchors**: Computes and overlays a perpendicular Start/Finish line (16-meter span) at the track's first coordinate.
* **Smart Turn Annotation**: Detects midpoint coordinates of sectors and dynamically offsets turn numbers (e.g. `1`, `2`, `3`...) outward by 15 meters along the path normal vector to avoid overlays and line overlap.

###  Graphing & Playback
* Multi-lane telemetry data visualization powered by `egui_plot` and `polars`.
* Synchronized time/distance scrub cursor, pan, and zoom boundaries across all active channels.
* An electric orange dot playback tracker coordinates real-time position updates on the track map as the user scrubs through telemetry data.

<img width="640" height="360" alt="makeintogif" src="https://github.com/user-attachments/assets/dd33138e-c0a8-4d8b-8484-cadb1aec24da" />


### 3. Automated Sector Detection
* Automated corner and sector detection using spatial telemetry.
* Comprehensive split times reporting table.
* Eclectic optimal lap calculation (combining fastest sector splits across the entire session).

### 4. Obsidian-Themed User Interface
* Customized dark user interface layout with a clean two-column page organization on both the **Dashboard** and **Reports** views.
* Rebuilt the **Dashboard Lap Sheet** into a 4-column striped grid to efficiently use panel space and increase visibility up to `240.0` max height.
* **Selection Outlines**: Replaced high-fill selection labels with high-contrast, 1px border frames (Orange, Cyan, White) to preserve text contrast and legibility.

---

## Tooling & Utilities

### Vector Exporter
* **`export_svg`**: An offline binary utility in [export_svg.rs](file:///D:/gtec/src/bin/export_svg.rs) that accepts any target `.ibt` file path and outputs a high-resolution, lightweight projected SVG track map for branding and graphics design.

#### CLI Usage & Examples
The utility parses a local iRacing `.ibt` telemetry file, projects the fastest lap's coordinates relative to the lap start, and exports a scaled, clean SVG vector layout.

**Command-line syntax:**
```bash
export_svg <input_ibt_file_path> <output_svg_file_path>
```

* **Windows (Command Prompt / PowerShell)**:
  ```powershell
  # Run the standalone executable:
  .\export_svg.exe "C:\Telemetry\session.ibt" "C:\Telemetry\track_layout.svg"
  ```
* **WSL / Linux**:
  ```bash
  # Run the compiled binary:
  ./export_svg "/mnt/c/Telemetry/session.ibt" "./track_layout.svg"
  ```
* **Running from Source (Cargo)**:
  ```bash
  cargo run --bin export_svg "D:\Oldtelems\session.ibt" "./track_layout.svg"
  ```

*Note: If no arguments are provided, the utility defaults to processing the pre-configured Sebring telemetry track log and outputs `sebring_vector_branding.svg` in the current working directory.*

---

## Getting Started
To compile and run this release, clone the repo and run:
```bash
cargo run --release
```

*For more details on structure and license terms, refer to the [README.md](file:///D:/gtec/README.md) and [LICENSE](file:///D:/gtec/LICENSE).*
