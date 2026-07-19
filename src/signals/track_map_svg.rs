use std::fs;
use std::path::Path;
use usvg::{Tree, Options};
use tiny_skia::Pixmap;

#[derive(Debug, Clone)]
pub struct SvgTrackMap {
    // Rendered image RGBA bytes
    pub rgba_bytes: Vec<u8>,
    pub width: usize,
    pub height: usize,
    
    // Bounds of the geometry path in the SVG document
    pub path_bbox: [f64; 4], // [min_x, min_y, max_x, max_y]
    // The full viewbox of the SVG
    pub view_box: [f64; 4],
}

fn traverse_group(group: &usvg::Group, min_x: &mut f32, min_y: &mut f32, max_x: &mut f32, max_y: &mut f32, found: &mut bool) {
    for node in group.children() {
        match node {
            usvg::Node::Path(ref p) => {
                let bbox = p.abs_bounding_box();
                if bbox.width() > 0.0 && bbox.height() > 0.0 {
                    *min_x = (*min_x).min(bbox.left());
                    *min_y = (*min_y).min(bbox.top());
                    *max_x = (*max_x).max(bbox.right());
                    *max_y = (*max_y).max(bbox.bottom());
                    *found = true;
                }
            }
            usvg::Node::Group(ref g) => {
                traverse_group(g, min_x, min_y, max_x, max_y, found);
            }
            _ => {}
        }
    }
}

pub fn load_and_render_svg_track_map(track_id: i32) -> Option<SvgTrackMap> {
    let svg_path = format!("assets/tracks/{}.svg", track_id);
    if !Path::new(&svg_path).exists() {
        return None;
    }
    
    let raw_data = fs::read_to_string(&svg_path).ok()?;
    
    // Replace white stroke with our sleek blue-grey!
    // The SVGs generally use 'stroke: white' or 'stroke="#FFF"'
    let svg_str = raw_data.replace("stroke: white", "stroke: #3b82f6").replace("stroke:white", "stroke:#3b82f6");
    let svg_data = svg_str.as_bytes();
    
    // Parse SVG
    let opt = Options::default();
    let tree = Tree::from_data(&svg_data, &opt).ok()?;
    
    // Find path bounding box
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    
    let mut found_path = false;
    traverse_group(tree.root(), &mut min_x, &mut min_y, &mut max_x, &mut max_y, &mut found_path);
    
    let view_box = [
        0.0,
        0.0,
        tree.size().width() as f64,
        tree.size().height() as f64,
    ];

    // If no valid path found, fallback to view_box
    let path_bbox = if found_path {
        [min_x as f64, min_y as f64, max_x as f64, max_y as f64]
    } else {
        view_box
    };

    
    // Render the SVG to a Pixmap.
    // To make it sharp, we can render it at 2000px width.
    let base_width = tree.size().width();
    let base_height = tree.size().height();
    
    let target_width = 2000.0_f32;
    let scale = target_width / base_width;
    let render_w = target_width;
    let render_h = base_height * scale;
    
    let mut pixmap = Pixmap::new(render_w as u32, render_h as u32)?;
    resvg::render(&tree, usvg::Transform::from_scale(scale as f32, scale as f32), &mut pixmap.as_mut());
    
    // `tiny_skia::Pixmap` has pixels as premultiplied RGBA `[u8]`
    let rgba_bytes = pixmap.data().to_vec();
    
    Some(SvgTrackMap {
        rgba_bytes,
        width: render_w as usize,
        height: render_h as usize,
        path_bbox,
        view_box,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_render() {
        if let Some(map) = load_and_render_svg_track_map(127) {
            println!("Rendered! W: {}, H: {}, Bbox: {:?}", map.width, map.height, map.path_bbox);
            let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                map.width as u32,
                map.height as u32,
                map.rgba_bytes
            ).unwrap();
            img.save("C:/Users/bukar/.gemini/antigravity/brain/c8b47878-7349-4735-8e99-08390eee4627/test_render_127.png").unwrap();
        }
    }
}
