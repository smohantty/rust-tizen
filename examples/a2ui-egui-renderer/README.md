# a2ui-egui-renderer

Experimental A2UI Basic Catalog renderer for Tizen using the same GPU path as
`examples/hello-egui-gpu`: `tizen-window` -> `tizen-egl` -> GLES -> `egui_glow`.

## Plan

1. Parse A2UI as data, not code. Accept JSONL streams, JSON arrays, single
   message objects, or sample wrapper objects with a `messages` array.
2. Maintain renderer state by `surfaceId`: catalog id, theme, component map,
   data model, and a root component id.
3. Normalize v0.9 Basic Catalog components into a flat `id -> component` map.
   A small v0.8 compatibility path is included for the older wrapper shape.
4. Resolve component references into an egui scene at frame time. Layout
   components recurse through `children`; controls resolve `path` bindings
   against the surface data model and write local edits back.
5. Dispatch user actions by printing a structured log line. There is no remote
   agent transport in this first-stage example.

The renderer follows the A2UI renderer model: components are a flat adjacency
list, data binding is resolved by JSON Pointer paths, and native widgets are
selected from a trusted catalog.

## Build

```sh
cd examples/a2ui-egui-renderer
cargo tizen build -A armv7l --release
```

## Run on target

```sh
rsdb agent fs mkdir --target 192.168.0.234 \
  /opt/usr/home/owner/rsdb-a2ui --parents --mode 755
rsdb agent transfer push --target 192.168.0.234 \
  target/tizen/armv7l/cargo/armv7-unknown-linux-gnueabi/release/a2ui-egui-renderer \
  /opt/usr/home/owner/rsdb-a2ui/a2ui-egui-renderer --verify sha256 --atomic --if-changed
rsdb agent transfer push --target 192.168.0.234 samples/contact_form.jsonl \
  /opt/usr/home/owner/rsdb-a2ui/contact_form.jsonl --verify sha256 --atomic --if-changed
rsdb agent exec --target 192.168.0.234 -- chmod +x \
  /opt/usr/home/owner/rsdb-a2ui/a2ui-egui-renderer
rsdb agent exec --target 192.168.0.234 --stream --timeout-secs 8 -- \
  sh -c 'XDG_RUNTIME_DIR=/run WAYLAND_DISPLAY=wayland-0 /opt/usr/home/owner/rsdb-a2ui/a2ui-egui-renderer /opt/usr/home/owner/rsdb-a2ui/contact_form.jsonl'
```

If no file path is passed, the built-in sample is rendered.
