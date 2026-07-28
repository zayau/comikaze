# comikaze

Comikaze generates hand-drawn comic-style SVG components from Rust or the
command line.

The library supports standalone comic panel frames and polygonal page layouts
with gutters and coordinated hand-drawn boundaries. The CLI supports standalone
frames and complete layouts described by JSON. Speech balloons and caption
boxes are planned.

## Command line

Generate a frame on standard output:

```console
cargo run -- frame --width 400 --height 600 --seed 42
```

Write it to a file:

```console
cargo run -- frame --seed 42 --output frame.svg
```

Frame colors support `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, and
`currentColor`. Supplying a seed makes the generated wobble reproducible.

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

## Library

### Standalone frame

```rust
use comikaze::frame::{FrameOptions, build_frame_svg};

let options = FrameOptions {
    width: 300,
    height: 400,
    color: "#202020".to_string(),
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

## Development

```console
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT
