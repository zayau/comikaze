# comikaze

Comikaze generates hand-drawn comic panel frames and complete page layouts as
SVG, from Rust or the command line.

The library supports standalone comic panel frames and polygonal page layouts
with gutters and coordinated hand-drawn boundaries. The CLI supports standalone
frames and complete layouts described by JSON. A filled standalone frame can
also be used as a caption box. Exact panel-mask exports let users clip their own
SVG artwork in Figma or another vector editor.

## Command line

Generate a frame on standard output:

```console
cargo run -- frame --width 400 --height 600 --seed 42
```

Write it to a file:

```console
cargo run -- frame --seed 42 --output frame.svg
```

Add an interior fill when using a frame as a box or another filled component:

```console
cargo run -- frame --width 320 --height 120 --fill '#fff8dc' --output box.svg
```

Frame colors support `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, and
`currentColor`. Frames are transparent unless `--fill` is supplied. Supplying
a seed makes the generated wobble reproducible.

Generate a complete layout from a JSON specification:

```console
cargo run -- layout examples/layout.json --output layout.svg
```

Omit `--output` to print the SVG on standard output. A layout begins with one
panel named `page`. Each ordered cut consumes its `target` panel and creates
named `negative` and `positive` panels that later cuts can target. Reversing a
cut's `start` and `end` points swaps those two sides.

See [`examples/layout.json`](examples/layout.json) for a complete six-panel
specification with gutters and coordinated hand-drawn rendering.

Export all final panel contours as an aligned, full-page mask overview:

```console
cargo run -- layout examples/layout.json --masks --output masks.svg
```

The mask SVG contains one independent black path per panel. Each path keeps its
JSON panel name in both its `id` and `data-panel-name` attributes—for example,
`panel-mask-top_right`.

The full-page SVG keeps the original page viewport so all masks remain aligned.
After importing it into Figma, ungroup the SVG and select the individual named
panel path—not the outer SVG wrapper—before choosing **Use as mask**. Use an
Alpha mask so the transparent page margin does not become part of the visible
region.

For a simpler Figma import, export one named panel as a tightly cropped,
single-path SVG:

```console
cargo run -- layout examples/layout.json \
  --mask bottom \
  --output bottom-mask.svg
```

This removes the surrounding page margin from the SVG viewport, preventing it
from being mistaken for the mask boundary. Because the single mask is cropped
out of the full page, position it over the corresponding panel after importing
it.

Panel names may contain ASCII letters, digits, `-`, and `_`. The mask contours
reuse the same gutter, inset, and coordinated hand-drawn calculations as the
frame renderer, so they match the generated frame outlines exactly.

## Library

### Standalone frame

```rust
use comikaze::frame::{FrameOptions, build_frame_svg};

let options = FrameOptions {
    width: 300,
    height: 400,
    color: "#202020".to_string(),
    fill: Some("#fff8dc".to_string()),
    stroke_width: 3.0,
    seed: Some(42),
};

let svg = build_frame_svg(&options).expect("valid frame options");
```

`build_frame_svg` validates its options and returns a typed `FrameError` when
the configuration is invalid.

### Irregular page layout

```rust
use comikaze::geometry::Point;
use comikaze::layout::{
    HandDrawnOptions,
    LayoutSvgOptions,
    PageLayout,
    build_layout_svg,
};

let mut layout =
    PageLayout::new(400, 600).expect("valid page dimensions");

// Separate the top row from the lower panel.
layout
    .split_panel(
        0,
        Point::new(0.0, 220.0),
        Point::new(400.0, 200.0),
    )
    .expect("cut should split the page");

// Split the top row into two irregular panels.
layout
    .split_panel(
        0,
        Point::new(210.0, 0.0),
        Point::new(190.0, 220.0),
    )
    .expect("cut should split the top row");

let options = LayoutSvgOptions {
    stroke_width: 3.0,
    gutter: 9.0,
    hand_drawn: Some(HandDrawnOptions {
        seed: 42,
        ..HandDrawnOptions::default()
    }),
    ..LayoutSvgOptions::default()
};

let svg =
    build_layout_svg(&layout, &options)
        .expect("valid layout options");
```

Shared panel boundaries reuse the same generated noise profile. This keeps
neighboring hand-drawn edges coordinated, including at T-junctions.
Use `build_layout_path_data(&layout, &options)` when you need the exact final
panel contours for custom masks or clipping rather than a complete SVG
document.

## Development

```console
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT
