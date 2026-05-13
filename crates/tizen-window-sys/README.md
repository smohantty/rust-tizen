# tizen-window-sys

Scanner-generated raw client bindings for the Wayland protocols a
Tizen native window uses:

| Protocol | Source | Role |
|---|---|---|
| `zxdg_shell_v6` | upstream wayland-protocols (vendored) | Window-management role (Tizen does not yet ship modern stable `xdg_wm_base`) |
| `wtz_shell` / `wtz_surface` | Tizen `wayland-extension` (vendored) | Second role on the same `wl_surface`; decoration + screen-assignment events |
| `wtz_screen` | Tizen `wayland-extension` (vendored) | Referenced by `wtz_surface.screen` |

The Tizen v6 choice is canonical — confirmed by reading
`tizen-core-wayland`'s source (the EFL-free reimplementation of
`ecore_wl2`): `tizen_core_wl_surface.c` calls
`zxdg_shell_v6_get_xdg_surface` + `zxdg_surface_v6_get_toplevel`
on every Tizen window.

Most users want the safe wrapper:
[`tizen-window`](../tizen-window/).
