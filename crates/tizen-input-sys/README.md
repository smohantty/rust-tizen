# tizen-input-sys

Raw client bindings for Tizen's `tizen_input_device_manager` Wayland
protocol, generated from a vendored slice of upstream `tizen-extension.xml`
via `wayland-scanner` at build time.

Most users want the safe wrapper: [`tizen-input`](../tizen-input/).

## Vendored XML

`protocols/tizen-input-device-manager.xml` is a trimmed extract of
`protocol/tizen/tizen-extension.xml` from the Tizen `wayland-extension`
repository. The exact upstream commit is recorded in
[`protocols/UPSTREAM.txt`](protocols/UPSTREAM.txt). The original MIT
license is preserved in [`protocols/LICENSE`](protocols/LICENSE).

To track a newer protocol version, copy the matching interface block
from upstream into the vendored XML, update `UPSTREAM.txt`, and rerun
`cargo check -p tizen-input-sys`.

## Runtime dependency

`libwayland-client.so.0` — already present on every Tizen device. It is
dlopened at runtime by `wayland-backend`, so this crate adds no
build-time link dependency on the rootstrap.
