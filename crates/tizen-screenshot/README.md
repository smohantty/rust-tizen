# tizen-screenshot

Capture screenshots from a Tizen Wayland compositor.

```rust
use tizen_screenshot::ScreenCapturer;

let mut cap = ScreenCapturer::new()?;
let frame = cap.shoot()?;
println!("{}x{} stride={} {} bytes",
    frame.width(), frame.height(), frame.stride(), frame.bytes().len());
```

The returned `ScreenFrame` owns its pixel bytes — the underlying TBM
buffer is mapped, copied out, and freed before the call returns.

## Format

Pixels come back as raw **`XRGB8888` little-endian** (4 bytes per
pixel; alpha byte unused). Rows are `stride` bytes apart, which may be
larger than `width * 4` on outputs with non-trivial row alignment.

The crate intentionally does **no** encoding — encode to PNG/JPEG/etc.
yourself (e.g. with the [`png`](https://crates.io/crates/png) crate)
or write the raw bytes for downstream conversion.

## Runtime deps

| Library | Resolution |
|---|---|
| `libwayland-client.so.0` | dlopen by `wayland-backend` |
| `libtbm.so.1` | linked directly under `cfg(tizen)` |
| `libwayland-tbm-client.so.0` | dlopen at first use (not in rootstrap) |

All three are present on every Tizen device.

## Limitations (initial slice)

* **One-shot only.** `tizen_screenmirror` (streaming captures) is a
  separate feature for a follow-up crate / cargo feature.
* **First output only.** Multi-monitor selection isn't exposed yet;
  the crate captures the first `wl_output` the compositor advertises,
  matching `efl_util` behaviour.
* **`tizen_screenshooter` v3+.** The compositor must advertise at
  least version 3. v4+ is required for `shoot_area`.
