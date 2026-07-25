fn main() {
    let mut _plot = egui_plot::Plot::new("test")
        .link_axis("my_link_group", [true, false])
        .link_cursor("my_cursor_group", [true, false]);
}
