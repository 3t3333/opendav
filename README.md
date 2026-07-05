<img width="2048" height="1152" alt="opendav-8(4)" src="https://github.com/user-attachments/assets/593cd024-e92b-4528-8282-c5c547418c81" />



# OpenDAV

Open, performance-centered telemetry 

Built natively in Rust using `egui` and `polars`, OpenDAV provides a fast, zero-allocation visual interface to parse, visualize, and analyze telemetry streams.

---

## Examples

### Graphs
<img width="2558" height="1375" alt="workspace_screenshot" src="https://github.com/user-attachments/assets/b3b7f386-0a52-4392-a6bf-966b37984c43" />




<img width="640" height="360" alt="makeintogif" src="https://github.com/user-attachments/assets/045c243c-3da1-466a-bf00-92e9896cc33c" />


### Sector Reports
<img width="2560" height="730" alt="reports_screenshot" src="https://github.com/user-attachments/assets/ee86c2dd-fb56-4bd5-8a17-50ae58a198c5" />







---


## Core Pillars

* **High-Performance Visualization**: extremely fast multi-lane graphing, featuring synchronized panning, zooming, and cursor scrub alignment.
* **GPS-Derived 2D Vector Maps**: Projects raw spherical GPS latitude and longitude telemetry directly into local Cartesian coordinates in meters, rendering interactive, zoomable track layouts.
* **Automated Sector Intelligence**: On-the-fly corner detection, split timing calculations, and eclectic optimal lap timing analysis.
* **Modern Design System**: Sleek, high-contrast Obsidian-themed visual system engineered for readability in both dark and light environments.

---

## Getting Started

### Prerequisites
To build OpenDAV from source, you will need the Rust toolchain installed:
* [Install Rust](https://www.rust-lang.org/tools/install)

### Building and Running
Clone the repository and run the cargo release build:
```bash
cargo run --release
```

Once running, select **📂 Load IBT Telemetry** from the top taskbar to load an iRacing `.ibt` telemetry log.

---

## Project Status

OpenDav is great for comparing basic input telemetry and analyzing sector times, and will expand more in the future. 
OpenDAV is currently in active pre-release development (`v0.1.0-rs`). The architecture, layout, and feature set are subject to rapid iteration. Contributions, issue reports, and feedback are welcome.

---

## License
Licensed under the Apache 2.0 License. See [LICENSE](LICENSE) for details.
