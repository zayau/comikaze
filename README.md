# comikaze

Comikaze generates hand-drawn comic-style SVG components from Rust or the
command line.

The current release supports comic panel frames. Speech balloons and caption
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

## Library

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

## Development

```console
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT
