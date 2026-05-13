# tizen-screenshot-sys

Raw client bindings for Tizen's screenshot path: the
`tizen_screenshooter` Wayland protocol plus the slice of
`libtbm.so` / `libwayland-tbm-client.so` that screenshot capture needs.

Most users want the safe wrapper:
[`tizen-screenshot`](../tizen-screenshot/).

## Runtime dependencies

| Library | Resolution | Notes |
|---|---|---|
| `libwayland-client.so.0` | dlopen by `wayland-backend` | Already in every Tizen build |
| `libtbm.so.1` | linked directly via `#[link(name = "tbm")]` under `cfg(tizen)` | In the Tizen Studio rootstrap |
| `libwayland-tbm-client.so.0` | `libloading::Library::new(...)` at first use | **Not** in the rootstrap; lives on the device |

`libwayland-tbm-client.so.0` is dlopened lazily so the rootstrap stays
clean — there is no build-time link dependency on it. The first call
into [`wayland_tbm::*`] loads the `.so` and caches the function table.

## Vendored XML

`protocols/tizen-screenshooter.xml` is a trimmed extract of
`protocol/tizen/tizen-extension.xml` from the Tizen
`wayland-extension` repository. The exact upstream commit is recorded
in [`protocols/UPSTREAM.txt`](protocols/UPSTREAM.txt). The streaming
`tizen_screenmirror` sibling is intentionally omitted from this
initial slice.
