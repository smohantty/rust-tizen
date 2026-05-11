# tizen-app-sys

Raw FFI bindings for Tizen's app framework — `ui_app_main` lifecycle and
`app_control` intents.

`#![no_std]`, no deps beyond `core`. For a safe wrapper, see
[`tizen-app`](../tizen-app).

## What's exposed

```rust
extern "C" {
    pub fn ui_app_main(argc, argv, callback, user_data) -> c_int;
    pub fn ui_app_exit();

    pub fn app_control_clone(clone, app_control) -> c_int;
    pub fn app_control_destroy(app_control) -> c_int;
    pub fn app_control_get_operation(app_control, operation) -> c_int;
    pub fn app_control_get_uri(app_control, uri) -> c_int;
    pub fn app_control_get_app_id(app_control, app_id) -> c_int;
    pub fn app_control_get_extra_data(app_control, key, value) -> c_int;
}
```

## Linking

Under `cfg(tizen)` this crate links `libcapi-appfw-application.so` and
`libcapi-appfw-app-control.so`. Off-target the declarations exist but no
library is requested, so `cargo check` works without a sysroot.

## License

Dual-licensed under Apache-2.0 OR MIT.
