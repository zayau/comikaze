use comikaze::geometry::Point;
use comikaze::layout::{LayoutSvgOptions, PageLayout, build_layout_svg};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut layout = PageLayout::new(600, 900)?;

    // Separate the top row from the rest of the page.
    layout.split_panel(0, Point::new(0.0, 270.0), Point::new(600.0, 300.0))?;

    // Separate the middle row from the bottom panel.
    layout.split_panel(1, Point::new(0.0, 620.0), Point::new(600.0, 590.0))?;

    // Split the top row into two panels.
    layout.split_panel(0, Point::new(330.0, 0.0), Point::new(300.0, 300.0))?;

    // Split the middle row, which is now panel 2.
    layout.split_panel(2, Point::new(200.0, 270.0), Point::new(230.0, 620.0))?;

    // Split its right-hand section again, creating three middle panels.
    layout.split_panel(2, Point::new(430.0, 270.0), Point::new(450.0, 620.0))?;

    let svg = build_layout_svg(
        &layout,
        &LayoutSvgOptions {
            stroke_width: 3.0,
            gutter: 12.0,
            ..LayoutSvgOptions::default()
        },
    )?;

    print!("{svg}");

    Ok(())
}
