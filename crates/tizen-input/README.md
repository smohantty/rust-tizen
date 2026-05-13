# tizen-input

Inject key/touch/pointer events into a Tizen Wayland compositor.

```rust
use tizen_input::{DeviceType, InputGenerator, KeyState};

let mut gen = InputGenerator::builder()
    .name("my-tester")
    .device(DeviceType::KEYBOARD)
    .open()?;

gen.key("XF86Back", KeyState::Pressed)?;
gen.key("XF86Back", KeyState::Released)?;
```

## How it works

Talks the `tizen_input_device_manager` Wayland protocol directly via
`wayland-client` (no EFL/Ecore/Elementary at runtime). The backend is
`client_system` + `dlopen` so `libwayland-client.so.0` is resolved at
runtime from the device — no build-time link dependency on the
rootstrap.

## Runtime dependency

`libwayland-client.so.0` only — present on every Tizen device.

## Off-target builds

`cargo check`/`cargo test` work on plain Linux. On a host without
`$WAYLAND_DISPLAY` set, `InputGenerator::open()` succeeds but returns
a no-op generator (each call writes a one-line trace to stderr) so
dev loops don't require a device.
