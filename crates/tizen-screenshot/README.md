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

## Known issue: server-side gate on some Tizen builds

On at least one Tizen 11 TV target (`Tizen10/TV` profile) the binding's
protocol traffic is **bit-for-bit identical to `efl_util_screenshot.c`**
(verified via `WAYLAND_DEBUG=client`) — registry walk, screenshooter
bind, `wayland_tbm_client_init`, `tbm_surface_create`, `wl_tbm.
create_buffer_with_fd`, then `tizen_screenshooter.shoot(output, buffer)`.
The compositor accepts the request, replies with
`screenshooter_notify(1)` (= permission granted per
`efl_util_screenshot.c:284`) and the supported `format` events, but
then **never emits `done`**. Both `EventQueue::roundtrip` and
`blocking_dispatch` wait loops time out indefinitely.

On the same target, Enlightenment's own debug tool
(`winfo -dump_screen`) captures successfully via a different
mechanism, so the hardware/compositor are capable.

The most likely cause is an app-context check (smack label, cgroup,
or parent-pid) the compositor enforces on `tizen_screenshooter.shoot`
beyond what `screenshooter_notify` reports — `rsdb`-launched binaries
running as root don't satisfy that gate. The binding works on targets
that do not enforce this gate; it is correct at the protocol level.

Future investigation: try the basic `screenshooter` protocol (also
advertised on these targets) as a fallback, or run the binary inside
an app-control sandbox.
