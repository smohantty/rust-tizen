# tizen-tbm-sys

Raw FFI bindings for `libtbm.so.1` (Tizen Buffer Manager) and
`libwayland-tbm-client.so.0` (Wayland buffer wrapper).

Higher-level Tizen Wayland crates depend on this:

| Consumer | Uses |
|---|---|
| [`tizen-screenshot-sys`](../tizen-screenshot-sys/) | TBM for screenshooter output buffers |
| [`tizen-window-sys`](../tizen-window-sys/) | TBM for window pixel buffers (Tizen apps use `wl_tbm` rather than `wl_shm` by convention) |

## Library availability

| Library | Rootstrap | Resolution |
|---|---|---|
| `libtbm.so.1` | yes | `#[cfg_attr(tizen, link(name = "tbm", kind = "dylib"))]` |
| `libwayland-tbm-client.so.0` | no | `libloading::Library::new(...)` at first use |

Both are present on every Tizen device.
