//! A2UI Basic Catalog renderer on Tizen, using egui over EGL/GLES.
//!
//! This is a first-stage renderer example: it accepts an A2UI JSON or JSONL
//! file, builds the A2UI surface state, resolves component IDs and data paths,
//! then renders the scene through egui. It intentionally treats A2UI as data:
//! unknown components render as diagnostic labels instead of executing code.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs;
use std::ptr;
use std::sync::Arc;
use std::time::Instant;

use glow::HasContext;
use khronos_egl as egl;
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use serde_json::{Map, Number, Value};
use tizen::egl::EglWindow;
use tizen::window::{
    Display, Event as TizenEvent, ModifiersState, MouseButton as TizenMouseButton, WindowBuilder,
};

const BUILTIN_SAMPLE: &str = include_str!("../samples/contact_form.jsonl");
const DEFAULT_SURFACE_ID: &str = "main";

type BoxError = Box<dyn std::error::Error>;

fn main() -> std::process::ExitCode {
    let launch = Instant::now();
    mark(launch, "main entered");

    if let Err(e) = run(launch) {
        eprintln!("a2ui-egui-renderer: {e}");
        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}

fn mark(launch: Instant, label: &str) {
    let ms = launch.elapsed().as_secs_f64() * 1000.0;
    println!("[launch +{ms:>7.2} ms] {label}");
}

fn run(launch: Instant) -> Result<(), BoxError> {
    let source = env::args().nth(1);
    let source_label = source
        .clone()
        .unwrap_or_else(|| "<builtin contact_form.jsonl>".to_owned());
    let input = match source.as_deref() {
        Some(path) => fs::read_to_string(path)?,
        None => BUILTIN_SAMPLE.to_owned(),
    };
    let messages = parse_messages(&input)?;
    let mut renderer = A2uiRuntime::default();
    renderer.process_messages(messages)?;
    println!(
        "a2ui-egui-renderer: loaded {source_label}; surfaces={}",
        renderer.surfaces.len()
    );

    let mut display = Display::connect()?;
    mark(launch, "Display::connect");

    let mut window = WindowBuilder::new()
        .title("a2ui-egui-renderer")
        .app_id("rust.tizen.a2ui-egui-renderer")
        .size(1920, 1080)
        .build(&display)?;
    mark(launch, "WindowBuilder::build");

    display.roundtrip(&mut window)?;
    let (init_w, init_h) = window.size();
    mark(launch, &format!("first configure ({init_w}x{init_h})"));

    // SAFETY: `window` lives for the rest of `run`, and `egl_window` drops first.
    let egl_window = unsafe { EglWindow::new(&window, init_w, init_h)? };
    mark(launch, "EglWindow::new");

    let egl_lib = unsafe { egl::DynamicInstance::<egl::EGL1_4>::load_required()? };
    mark(launch, "libEGL loaded");

    let wl_display_ptr = match display
        .display_handle()
        .map_err(|e| format!("display_handle: {e}"))?
        .as_raw()
    {
        RawDisplayHandle::Wayland(wl) => wl.display.as_ptr(),
        _ => return Err("display handle is not Wayland".into()),
    };
    let egl_display = unsafe {
        egl_lib
            .get_display(wl_display_ptr)
            .ok_or("eglGetDisplay returned NO_DISPLAY")?
    };
    let (egl_major, egl_minor) = egl_lib.initialize(egl_display)?;
    mark(launch, &format!("eglInitialize ({egl_major}.{egl_minor})"));
    egl_lib.bind_api(egl::OPENGL_ES_API)?;

    let cfg_attrs = [
        egl::SURFACE_TYPE,
        egl::WINDOW_BIT,
        egl::RED_SIZE,
        8,
        egl::GREEN_SIZE,
        8,
        egl::BLUE_SIZE,
        8,
        egl::ALPHA_SIZE,
        0,
        egl::DEPTH_SIZE,
        0,
        egl::STENCIL_SIZE,
        0,
        egl::RENDERABLE_TYPE,
        egl::OPENGL_ES2_BIT,
        egl::NONE,
    ];
    let config = egl_lib
        .choose_first_config(egl_display, &cfg_attrs)?
        .ok_or("no matching EGL config")?;

    let (context, gl_major) = match try_context(&egl_lib, egl_display, config, 3) {
        Ok(c) => (c, 3),
        Err(_) => (try_context(&egl_lib, egl_display, config, 2)?, 2),
    };
    mark(launch, &format!("eglCreateContext (GLES {gl_major})"));

    let egl_surface =
        unsafe { egl_lib.create_window_surface(egl_display, config, egl_window.as_ptr(), None)? };
    egl_lib.make_current(
        egl_display,
        Some(egl_surface),
        Some(egl_surface),
        Some(context),
    )?;
    mark(launch, "eglMakeCurrent");

    let gl = Arc::new(unsafe {
        glow::Context::from_loader_function(|name| {
            egl_lib
                .get_proc_address(name)
                .map(|p| p as *const _)
                .unwrap_or(ptr::null())
        })
    });
    mark(launch, "glow::Context");

    let mut painter = egui_glow::Painter::new(gl.clone(), "", None, false)
        .map_err(|e| format!("egui_glow::Painter::new: {e}"))?;
    mark(launch, "egui_glow::Painter");

    let egui_ctx = egui::Context::default();
    install_tv_style(&egui_ctx);

    let start = Instant::now();
    let mut frame: u64 = 0;
    let mut last_log = Instant::now();
    let mut frames_since_log: u64 = 0;
    let mut first_frame_done = false;
    let mut input = EguiInputBridge::default();
    window.request_redraw();

    display.run(&mut window, |window, event| match event {
        TizenEvent::Resized { width, height } => {
            let _ = egl_window.resize(width, height, 0, 0);
            window.request_redraw();
        }
        TizenEvent::RedrawRequested => {
            let (w, h) = window.size();
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(w as f32, h as f32),
                )),
                time: Some(start.elapsed().as_secs_f64()),
                modifiers: input.modifiers,
                events: input.take_events(),
                focused: input.focused,
                ..Default::default()
            };

            let full = egui_ctx.run(raw_input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    renderer.render(ui);
                });
            });

            if !first_frame_done {
                mark(launch, "first A2UI egui pass");
            }
            let primitives = egui_ctx.tessellate(full.shapes, full.pixels_per_point);

            unsafe {
                gl.viewport(0, 0, w as i32, h as i32);
                gl.clear_color(0.04, 0.045, 0.055, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
            }
            painter.paint_and_update_textures(
                [w, h],
                full.pixels_per_point,
                &primitives,
                &full.textures_delta,
            );

            let _ = egl_lib.swap_buffers(egl_display, egl_surface);

            if !first_frame_done {
                let ms = launch.elapsed().as_secs_f64() * 1000.0;
                mark(launch, "first eglSwapBuffers returned");
                println!("a2ui-egui-renderer: first_frame_ms = {ms:.2}");
                first_frame_done = true;
            }

            frame += 1;
            frames_since_log += 1;
            if last_log.elapsed().as_secs_f32() >= 1.0 {
                let fps = frames_since_log as f32 / last_log.elapsed().as_secs_f32();
                println!("a2ui-egui-renderer: frame={frame} fps~{fps:.1}");
                frames_since_log = 0;
                last_log = Instant::now();
            }

            window.request_redraw();
        }
        TizenEvent::CloseRequested => {}
        other => {
            if input.push_tizen_event(other) {
                window.request_redraw();
            }
        }
    })?;

    Ok(())
}

fn try_context(
    egl_lib: &egl::DynamicInstance<egl::EGL1_4>,
    egl_display: egl::Display,
    config: egl::Config,
    client_version: i32,
) -> Result<egl::Context, BoxError> {
    let attrs = [egl::CONTEXT_CLIENT_VERSION, client_version, egl::NONE];
    Ok(egl_lib.create_context(egl_display, config, None, &attrs)?)
}

fn install_tv_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(14.0, 12.0);
    style.spacing.button_padding = egui::vec2(16.0, 10.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(47, 111, 237);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(58, 128, 250);
    ctx.set_style(style);
}

#[derive(Default)]
struct EguiInputBridge {
    events: Vec<egui::Event>,
    pointer_pos: Option<egui::Pos2>,
    modifiers: egui::Modifiers,
    focused: bool,
}

impl EguiInputBridge {
    fn take_events(&mut self) -> Vec<egui::Event> {
        std::mem::take(&mut self.events)
    }

    fn push_tizen_event(&mut self, event: TizenEvent) -> bool {
        match event {
            TizenEvent::Focused(focused) => {
                self.focused = focused;
                self.events.push(egui::Event::WindowFocused(focused));
                true
            }
            TizenEvent::CursorEntered { x, y } | TizenEvent::CursorMoved { x, y } => {
                let pos = egui::pos2(x as f32, y as f32);
                self.pointer_pos = Some(pos);
                self.events.push(egui::Event::PointerMoved(pos));
                true
            }
            TizenEvent::CursorLeft => {
                self.pointer_pos = None;
                self.events.push(egui::Event::PointerGone);
                true
            }
            TizenEvent::MouseInput { button, pressed } => {
                if let Some(pos) = self.pointer_pos {
                    self.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui_button(button),
                        pressed,
                        modifiers: self.modifiers,
                    });
                    true
                } else {
                    false
                }
            }
            TizenEvent::MouseWheel { dx, dy } => {
                self.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(dx as f32, dy as f32),
                    modifiers: self.modifiers,
                });
                true
            }
            TizenEvent::KeyboardInput {
                keycode,
                pressed,
                modifiers,
            } => {
                self.modifiers = egui_modifiers(modifiers);
                if let Some(key) = egui_key(keycode) {
                    self.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: self.modifiers,
                    });
                }
                if pressed {
                    if let Some(text) = text_for_keycode(keycode, self.modifiers.shift) {
                        self.events.push(egui::Event::Text(text));
                    }
                }
                true
            }
            _ => false,
        }
    }
}

fn egui_button(button: TizenMouseButton) -> egui::PointerButton {
    match button {
        TizenMouseButton::Left => egui::PointerButton::Primary,
        TizenMouseButton::Right => egui::PointerButton::Secondary,
        TizenMouseButton::Middle => egui::PointerButton::Middle,
        _ => egui::PointerButton::Extra1,
    }
}

fn egui_modifiers(mods: ModifiersState) -> egui::Modifiers {
    egui::Modifiers {
        alt: mods.contains(ModifiersState::ALT),
        ctrl: mods.contains(ModifiersState::CTRL),
        shift: mods.contains(ModifiersState::SHIFT),
        mac_cmd: false,
        command: mods.contains(ModifiersState::CTRL) || mods.contains(ModifiersState::LOGO),
    }
}

fn egui_key(code: u32) -> Option<egui::Key> {
    Some(match code {
        1 => egui::Key::Escape,
        14 => egui::Key::Backspace,
        15 => egui::Key::Tab,
        28 => egui::Key::Enter,
        57 => egui::Key::Space,
        102 => egui::Key::Home,
        103 => egui::Key::ArrowUp,
        104 => egui::Key::PageUp,
        105 => egui::Key::ArrowLeft,
        106 => egui::Key::ArrowRight,
        107 => egui::Key::End,
        108 => egui::Key::ArrowDown,
        109 => egui::Key::PageDown,
        111 => egui::Key::Delete,
        2 => egui::Key::Num1,
        3 => egui::Key::Num2,
        4 => egui::Key::Num3,
        5 => egui::Key::Num4,
        6 => egui::Key::Num5,
        7 => egui::Key::Num6,
        8 => egui::Key::Num7,
        9 => egui::Key::Num8,
        10 => egui::Key::Num9,
        11 => egui::Key::Num0,
        16 => egui::Key::Q,
        17 => egui::Key::W,
        18 => egui::Key::E,
        19 => egui::Key::R,
        20 => egui::Key::T,
        21 => egui::Key::Y,
        22 => egui::Key::U,
        23 => egui::Key::I,
        24 => egui::Key::O,
        25 => egui::Key::P,
        30 => egui::Key::A,
        31 => egui::Key::S,
        32 => egui::Key::D,
        33 => egui::Key::F,
        34 => egui::Key::G,
        35 => egui::Key::H,
        36 => egui::Key::J,
        37 => egui::Key::K,
        38 => egui::Key::L,
        44 => egui::Key::Z,
        45 => egui::Key::X,
        46 => egui::Key::C,
        47 => egui::Key::V,
        48 => egui::Key::B,
        49 => egui::Key::N,
        50 => egui::Key::M,
        _ => return None,
    })
}

fn text_for_keycode(code: u32, shift: bool) -> Option<String> {
    let ch = match code {
        2 => {
            if shift {
                '!'
            } else {
                '1'
            }
        }
        3 => {
            if shift {
                '@'
            } else {
                '2'
            }
        }
        4 => {
            if shift {
                '#'
            } else {
                '3'
            }
        }
        5 => {
            if shift {
                '$'
            } else {
                '4'
            }
        }
        6 => {
            if shift {
                '%'
            } else {
                '5'
            }
        }
        7 => {
            if shift {
                '^'
            } else {
                '6'
            }
        }
        8 => {
            if shift {
                '&'
            } else {
                '7'
            }
        }
        9 => {
            if shift {
                '*'
            } else {
                '8'
            }
        }
        10 => {
            if shift {
                '('
            } else {
                '9'
            }
        }
        11 => {
            if shift {
                ')'
            } else {
                '0'
            }
        }
        12 => {
            if shift {
                '_'
            } else {
                '-'
            }
        }
        13 => {
            if shift {
                '+'
            } else {
                '='
            }
        }
        16 => letter('q', shift),
        17 => letter('w', shift),
        18 => letter('e', shift),
        19 => letter('r', shift),
        20 => letter('t', shift),
        21 => letter('y', shift),
        22 => letter('u', shift),
        23 => letter('i', shift),
        24 => letter('o', shift),
        25 => letter('p', shift),
        30 => letter('a', shift),
        31 => letter('s', shift),
        32 => letter('d', shift),
        33 => letter('f', shift),
        34 => letter('g', shift),
        35 => letter('h', shift),
        36 => letter('j', shift),
        37 => letter('k', shift),
        38 => letter('l', shift),
        39 => {
            if shift {
                ':'
            } else {
                ';'
            }
        }
        40 => {
            if shift {
                '"'
            } else {
                '\''
            }
        }
        41 => {
            if shift {
                '~'
            } else {
                '`'
            }
        }
        43 => {
            if shift {
                '|'
            } else {
                '\\'
            }
        }
        44 => letter('z', shift),
        45 => letter('x', shift),
        46 => letter('c', shift),
        47 => letter('v', shift),
        48 => letter('b', shift),
        49 => letter('n', shift),
        50 => letter('m', shift),
        51 => {
            if shift {
                '<'
            } else {
                ','
            }
        }
        52 => {
            if shift {
                '>'
            } else {
                '.'
            }
        }
        53 => {
            if shift {
                '?'
            } else {
                '/'
            }
        }
        57 => ' ',
        _ => return None,
    };
    Some(ch.to_string())
}

fn letter(ch: char, shift: bool) -> char {
    if shift {
        ch.to_ascii_uppercase()
    } else {
        ch
    }
}

#[derive(Default)]
struct A2uiRuntime {
    surfaces: BTreeMap<String, Surface>,
    active_surface: Option<String>,
    diagnostics: Vec<String>,
}

impl A2uiRuntime {
    fn process_messages(&mut self, messages: Vec<Value>) -> Result<(), BoxError> {
        for message in messages {
            self.process_message(message)?;
        }
        Ok(())
    }

    fn process_message(&mut self, message: Value) -> Result<(), BoxError> {
        if let Some(create) = message.get("createSurface").and_then(Value::as_object) {
            let surface_id = string_prop(create, "surfaceId").unwrap_or(DEFAULT_SURFACE_ID);
            let catalog_id = string_prop(create, "catalogId").unwrap_or("");
            let theme = create.get("theme").cloned().unwrap_or(Value::Null);
            let surface = self
                .surfaces
                .entry(surface_id.to_owned())
                .or_insert_with(|| Surface::new(surface_id));
            surface.catalog_id = catalog_id.to_owned();
            surface.theme = theme;
            surface.root = "root".to_owned();
            self.active_surface = Some(surface_id.to_owned());
            return Ok(());
        }

        if let Some(begin) = message.get("beginRendering").and_then(Value::as_object) {
            let surface_id = string_prop(begin, "surfaceId").unwrap_or(DEFAULT_SURFACE_ID);
            let catalog_id = string_prop(begin, "catalogId").unwrap_or("");
            let root = string_prop(begin, "root").unwrap_or("root");
            let theme = begin.get("styles").cloned().unwrap_or(Value::Null);
            let surface = self
                .surfaces
                .entry(surface_id.to_owned())
                .or_insert_with(|| Surface::new(surface_id));
            surface.catalog_id = catalog_id.to_owned();
            surface.theme = theme;
            surface.root = root.to_owned();
            self.active_surface = Some(surface_id.to_owned());
            return Ok(());
        }

        if let Some(update) = message
            .get("updateComponents")
            .or_else(|| message.get("surfaceUpdate"))
            .and_then(Value::as_object)
        {
            let surface_id = string_prop(update, "surfaceId").unwrap_or(DEFAULT_SURFACE_ID);
            let surface = self
                .surfaces
                .entry(surface_id.to_owned())
                .or_insert_with(|| Surface::new(surface_id));
            if let Some(components) = update.get("components").and_then(Value::as_array) {
                for raw in components {
                    match Component::from_value(raw) {
                        Ok(component) => surface.insert_component(component),
                        Err(e) => self
                            .diagnostics
                            .push(format!("component parse error on {surface_id}: {e}")),
                    }
                }
            }
            self.active_surface = Some(surface_id.to_owned());
            return Ok(());
        }

        if let Some(update) = message
            .get("updateDataModel")
            .or_else(|| message.get("dataModelUpdate"))
            .and_then(Value::as_object)
        {
            let surface_id = string_prop(update, "surfaceId").unwrap_or(DEFAULT_SURFACE_ID);
            let surface = self
                .surfaces
                .entry(surface_id.to_owned())
                .or_insert_with(|| Surface::new(surface_id));
            let path = string_prop(update, "path").unwrap_or("/");
            if let Some(value) = update.get("value") {
                set_json_path(&mut surface.data_model, path, value.clone());
            } else if let Some(contents) = update.get("contents").and_then(Value::as_array) {
                let converted = data_contents_to_value(contents);
                set_json_path(&mut surface.data_model, path, converted);
            } else {
                remove_json_path(&mut surface.data_model, path);
            }
            self.active_surface = Some(surface_id.to_owned());
            return Ok(());
        }

        if let Some(delete) = message.get("deleteSurface").and_then(Value::as_object) {
            let surface_id = string_prop(delete, "surfaceId").unwrap_or(DEFAULT_SURFACE_ID);
            self.surfaces.remove(surface_id);
            if self.active_surface.as_deref() == Some(surface_id) {
                self.active_surface = self.surfaces.keys().next_back().cloned();
            }
            return Ok(());
        }

        self.diagnostics
            .push("ignored unknown A2UI message".to_owned());
        Ok(())
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        let surface_id = match self.active_surface.clone() {
            Some(id) => id,
            None => {
                ui.heading("A2UI renderer");
                ui.label("No active surface.");
                for d in &self.diagnostics {
                    ui.colored_label(egui::Color32::YELLOW, d);
                }
                return;
            }
        };

        let Some(surface) = self.surfaces.get_mut(&surface_id) else {
            ui.label(format!("Surface `{surface_id}` is missing."));
            return;
        };

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width());
                let mut stack = Vec::new();
                let root = surface.root.clone();
                render_component(ui, surface, &root, &mut stack, None);
            });
    }
}

#[derive(Clone, Debug)]
struct Component {
    id: String,
    kind: String,
    props: Map<String, Value>,
}

impl Component {
    fn from_value(raw: &Value) -> Result<Self, String> {
        let obj = raw
            .as_object()
            .ok_or_else(|| "component is not an object".to_owned())?;
        let id = string_prop(obj, "id")
            .ok_or_else(|| "component is missing `id`".to_owned())?
            .to_owned();

        let component = obj
            .get("component")
            .ok_or_else(|| format!("component `{id}` is missing `component`"))?;

        if let Some(kind) = component.as_str() {
            let mut props = obj.clone();
            props.remove("id");
            props.remove("component");
            return Ok(Self {
                id,
                kind: kind.to_owned(),
                props,
            });
        }

        let wrapper = component
            .as_object()
            .ok_or_else(|| format!("component `{id}` has invalid `component`"))?;
        let (kind, inner) = wrapper
            .iter()
            .next()
            .ok_or_else(|| format!("component `{id}` wrapper is empty"))?;
        let mut props = inner
            .as_object()
            .cloned()
            .ok_or_else(|| format!("component `{id}` inner payload is not an object"))?;
        if let Some(weight) = obj.get("weight") {
            props.insert("weight".to_owned(), weight.clone());
        }
        Ok(Self {
            id,
            kind: kind.to_owned(),
            props,
        })
    }
}

#[derive(Debug)]
struct Surface {
    catalog_id: String,
    theme: Value,
    root: String,
    components: HashMap<String, Component>,
    component_order: Vec<String>,
    data_model: Value,
    local_values: HashMap<String, Value>,
    tabs: HashMap<String, usize>,
    modal_open: HashSet<String>,
}

impl Surface {
    fn new(_id: &str) -> Self {
        Self {
            catalog_id: String::new(),
            theme: Value::Null,
            root: "root".to_owned(),
            components: HashMap::new(),
            component_order: Vec::new(),
            data_model: Value::Object(Map::new()),
            local_values: HashMap::new(),
            tabs: HashMap::new(),
            modal_open: HashSet::new(),
        }
    }

    fn insert_component(&mut self, component: Component) {
        if !self.components.contains_key(&component.id) {
            self.component_order.push(component.id.clone());
        }
        self.components.insert(component.id.clone(), component);
    }
}

fn render_component(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    id: &str,
    stack: &mut Vec<String>,
    base_path: Option<&str>,
) {
    if stack.iter().any(|seen| seen == id) {
        ui.colored_label(egui::Color32::RED, format!("Cycle at component `{id}`"));
        return;
    }

    let Some(component) = surface.components.get(id).cloned() else {
        ui.colored_label(egui::Color32::YELLOW, format!("Missing component `{id}`"));
        return;
    };

    stack.push(id.to_owned());
    ui.push_id(
        component_identity(&component.id, base_path),
        |ui| match component.kind.as_str() {
            "Text" => render_text(ui, surface, &component, base_path),
            "Icon" => render_icon(ui, surface, &component, base_path),
            "Image" => render_media_placeholder(ui, surface, &component, "image", base_path),
            "Video" => render_media_placeholder(ui, surface, &component, "video", base_path),
            "AudioPlayer" => render_media_placeholder(ui, surface, &component, "audio", base_path),
            "Row" => render_row(ui, surface, &component, stack, base_path),
            "Column" => render_column(ui, surface, &component, stack, base_path),
            "List" => render_list(ui, surface, &component, stack, base_path),
            "Card" => render_card(ui, surface, &component, stack, base_path),
            "Tabs" => render_tabs(ui, surface, &component, stack, base_path),
            "Modal" => render_modal(ui, surface, &component, stack, base_path),
            "Divider" => render_divider(ui, &component),
            "Button" => render_button(ui, surface, &component, base_path),
            "TextField" => render_text_field(ui, surface, &component, base_path),
            "CheckBox" => render_checkbox(ui, surface, &component, base_path),
            "ChoicePicker" => render_choice_picker(ui, surface, &component, base_path),
            "Slider" => render_slider(ui, surface, &component, base_path),
            "DateTimeInput" => render_text_field(ui, surface, &component, base_path),
            other => {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("Unsupported component `{other}` ({})", component.id),
                );
            }
        },
    );
    stack.pop();
}

fn render_text(
    ui: &mut egui::Ui,
    surface: &Surface,
    component: &Component,
    base_path: Option<&str>,
) {
    let raw = resolve_string(surface, component.props.get("text"), base_path).unwrap_or_default();
    let text = strip_markdown_heading(&raw);
    let variant = string_prop(&component.props, "variant")
        .or_else(|| string_prop(&component.props, "usageHint"))
        .unwrap_or("body");

    let rich = match variant {
        "h1" => egui::RichText::new(text).size(42.0).strong(),
        "h2" => egui::RichText::new(text).size(34.0).strong(),
        "h3" => egui::RichText::new(text).size(28.0).strong(),
        "h4" => egui::RichText::new(text).size(24.0).strong(),
        "h5" => egui::RichText::new(text).size(21.0).strong(),
        "caption" => egui::RichText::new(text)
            .size(16.0)
            .color(egui::Color32::GRAY),
        _ => egui::RichText::new(text).size(20.0),
    };
    ui.label(rich);
}

fn render_icon(
    ui: &mut egui::Ui,
    surface: &Surface,
    component: &Component,
    base_path: Option<&str>,
) {
    let name = resolve_string(surface, component.props.get("name"), base_path)
        .unwrap_or_else(|| "icon".to_owned());
    ui.monospace(format!("[{}]", name));
}

fn render_media_placeholder(
    ui: &mut egui::Ui,
    surface: &Surface,
    component: &Component,
    label: &str,
    base_path: Option<&str>,
) {
    let url = resolve_string(surface, component.props.get("url"), base_path).unwrap_or_default();
    let description = resolve_string(surface, component.props.get("description"), base_path)
        .or_else(|| resolve_string(surface, component.props.get("altText"), base_path))
        .unwrap_or_default();
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new(format!("{label}: {url}")).monospace());
        if !description.is_empty() {
            ui.label(description);
        }
    });
}

fn render_row(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    stack: &mut Vec<String>,
    base_path: Option<&str>,
) {
    ui.horizontal_wrapped(|ui| {
        render_children(ui, surface, component, stack, base_path, true);
    });
}

fn render_column(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    stack: &mut Vec<String>,
    base_path: Option<&str>,
) {
    ui.vertical(|ui| {
        render_children(ui, surface, component, stack, base_path, false);
    });
}

fn render_list(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    stack: &mut Vec<String>,
    base_path: Option<&str>,
) {
    let horizontal = string_prop(&component.props, "direction") == Some("horizontal");
    if horizontal {
        ui.horizontal_wrapped(|ui| render_children(ui, surface, component, stack, base_path, true));
    } else {
        render_children(ui, surface, component, stack, base_path, false);
    }
}

fn render_card(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    stack: &mut Vec<String>,
    base_path: Option<&str>,
) {
    let child = string_prop(&component.props, "child").unwrap_or("");
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            render_component(ui, surface, child, stack, base_path)
        });
}

fn render_tabs(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    stack: &mut Vec<String>,
    base_path: Option<&str>,
) {
    let tabs = component
        .props
        .get("tabs")
        .or_else(|| component.props.get("tabItems"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if tabs.is_empty() {
        ui.colored_label(egui::Color32::YELLOW, "Tabs has no items");
        return;
    }

    let mut selected = surface.tabs.get(&component.id).copied().unwrap_or(0);
    if selected >= tabs.len() {
        selected = 0;
    }
    let titles: Vec<String> = tabs
        .iter()
        .enumerate()
        .map(|(idx, tab)| {
            tab.get("title")
                .and_then(|value| resolve_string(surface, Some(value), base_path))
                .unwrap_or_else(|| format!("Tab {}", idx + 1))
        })
        .collect();
    ui.horizontal_wrapped(|ui| {
        for (idx, title) in titles.iter().enumerate() {
            if ui.selectable_label(selected == idx, title).clicked() {
                selected = idx;
            }
        }
    });
    surface.tabs.insert(component.id.clone(), selected);
    ui.separator();

    let child = tabs
        .get(selected)
        .and_then(|tab| tab.get("child"))
        .and_then(Value::as_str)
        .unwrap_or("");
    render_component(ui, surface, child, stack, base_path);
}

fn render_modal(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    stack: &mut Vec<String>,
    base_path: Option<&str>,
) {
    let trigger = string_prop(&component.props, "trigger")
        .or_else(|| string_prop(&component.props, "entryPointChild"))
        .unwrap_or("");
    let content = string_prop(&component.props, "content")
        .or_else(|| string_prop(&component.props, "contentChild"))
        .unwrap_or("");
    let label = component_label(surface, trigger, base_path).unwrap_or_else(|| "Open".to_owned());
    if ui.button(label).clicked() {
        surface.modal_open.insert(component.id.clone());
    }

    if surface.modal_open.contains(&component.id) {
        let ctx = ui.ctx().clone();
        let mut open = true;
        egui::Window::new(component.id.clone())
            .open(&mut open)
            .show(&ctx, |ui| {
                render_component(ui, surface, content, stack, base_path)
            });
        if !open {
            surface.modal_open.remove(&component.id);
        }
    }
}

fn render_divider(ui: &mut egui::Ui, _component: &Component) {
    ui.separator();
}

fn render_button(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    base_path: Option<&str>,
) {
    let child = string_prop(&component.props, "child").unwrap_or("");
    let label = component_label(surface, child, base_path).unwrap_or_else(|| child.to_owned());
    let variant = string_prop(&component.props, "variant").unwrap_or("default");
    let clicked = if variant == "borderless" {
        ui.link(label).clicked()
    } else {
        let text = if variant == "primary" {
            egui::RichText::new(label).strong()
        } else {
            egui::RichText::new(label)
        };
        ui.add(egui::Button::new(text)).clicked()
    };

    if clicked {
        if let Some(action) = component.props.get("action") {
            dispatch_action(surface, action, base_path);
        }
    }

    for error in check_errors(surface, &component.props, base_path) {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}

fn render_text_field(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    base_path: Option<&str>,
) {
    let label =
        resolve_string(surface, component.props.get("label"), base_path).unwrap_or_default();
    let value_expr = component
        .props
        .get("value")
        .or_else(|| component.props.get("text"));
    let mut text = read_value_or_local(surface, &component.id, value_expr, base_path)
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_default();

    if !label.is_empty() {
        ui.label(
            egui::RichText::new(label)
                .size(16.0)
                .color(egui::Color32::GRAY),
        );
    }

    let variant = string_prop(&component.props, "variant")
        .or_else(|| string_prop(&component.props, "textFieldType"))
        .unwrap_or("shortText");
    let response = if variant == "longText" {
        ui.add(egui::TextEdit::multiline(&mut text).desired_rows(4))
    } else if variant == "obscured" {
        ui.add(egui::TextEdit::singleline(&mut text).password(true))
    } else {
        ui.text_edit_singleline(&mut text)
    };

    if response.changed() {
        write_value_or_local(
            surface,
            &component.id,
            value_expr,
            Value::String(text),
            base_path,
        );
    }

    for error in check_errors(surface, &component.props, base_path) {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}

fn render_checkbox(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    base_path: Option<&str>,
) {
    let label =
        resolve_string(surface, component.props.get("label"), base_path).unwrap_or_default();
    let value_expr = component.props.get("value");
    let mut value = read_value_or_local(surface, &component.id, value_expr, base_path)
        .as_bool()
        .unwrap_or(false);
    if ui.checkbox(&mut value, label).changed() {
        write_value_or_local(
            surface,
            &component.id,
            value_expr,
            Value::Bool(value),
            base_path,
        );
    }

    for error in check_errors(surface, &component.props, base_path) {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}

fn render_choice_picker(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    base_path: Option<&str>,
) {
    if let Some(label) = resolve_string(surface, component.props.get("label"), base_path) {
        if !label.is_empty() {
            ui.label(
                egui::RichText::new(label)
                    .size(16.0)
                    .color(egui::Color32::GRAY),
            );
        }
    }

    let value_expr = component.props.get("value");
    let mut selected = resolve_string_list(surface, value_expr, base_path).unwrap_or_else(|| {
        surface
            .local_values
            .get(&component.id)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    });
    let multiple = string_prop(&component.props, "variant") == Some("multipleSelection");
    let options = component
        .props
        .get("options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for option in options {
        let label = option
            .get("label")
            .and_then(|value| resolve_string(surface, Some(value), base_path))
            .unwrap_or_default();
        let value = option
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or(&label)
            .to_owned();

        if multiple {
            let mut checked = selected.iter().any(|item| item == &value);
            if ui.checkbox(&mut checked, label).changed() {
                if checked {
                    selected.push(value);
                } else {
                    selected.retain(|item| item != &value);
                }
            }
        } else {
            let mut is_selected = selected.first() == Some(&value);
            if ui.radio_value(&mut is_selected, true, label).changed() && is_selected {
                selected = vec![value];
            }
        }
    }

    let value = Value::Array(selected.into_iter().map(Value::String).collect());
    write_value_or_local(surface, &component.id, value_expr, value, base_path);

    for error in check_errors(surface, &component.props, base_path) {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}

fn render_slider(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    base_path: Option<&str>,
) {
    let label =
        resolve_string(surface, component.props.get("label"), base_path).unwrap_or_default();
    let value_expr = component.props.get("value");
    let mut value = resolve_number(surface, value_expr, base_path)
        .or_else(|| {
            surface
                .local_values
                .get(&component.id)
                .and_then(Value::as_f64)
        })
        .unwrap_or(0.0);
    let min = component
        .props
        .get("min")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let max = component
        .props
        .get("max")
        .and_then(Value::as_f64)
        .unwrap_or(100.0);
    if ui
        .add(egui::Slider::new(&mut value, min..=max).text(label))
        .changed()
    {
        if let Some(num) = Number::from_f64(value) {
            write_value_or_local(
                surface,
                &component.id,
                value_expr,
                Value::Number(num),
                base_path,
            );
        }
    }

    for error in check_errors(surface, &component.props, base_path) {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}

fn render_children(
    ui: &mut egui::Ui,
    surface: &mut Surface,
    component: &Component,
    stack: &mut Vec<String>,
    base_path: Option<&str>,
    add_spacing: bool,
) {
    match component.props.get("children") {
        Some(Value::Array(children)) => {
            for child in children.iter().filter_map(Value::as_str) {
                render_component(ui, surface, child, stack, base_path);
                if add_spacing {
                    ui.add_space(10.0);
                }
            }
        }
        Some(Value::Object(children)) => {
            if let Some(explicit) = children.get("explicitList").and_then(Value::as_array) {
                for child in explicit.iter().filter_map(Value::as_str) {
                    render_component(ui, surface, child, stack, base_path);
                    if add_spacing {
                        ui.add_space(10.0);
                    }
                }
            } else if let (Some(template), Some(path)) = (
                children.get("componentId").and_then(Value::as_str),
                children
                    .get("path")
                    .or_else(|| children.get("dataBinding"))
                    .and_then(Value::as_str),
            ) {
                let bases = template_base_paths(surface, path, base_path);
                for child_base in bases {
                    render_component(ui, surface, template, stack, Some(&child_base));
                    if add_spacing {
                        ui.add_space(10.0);
                    }
                }
            }
        }
        _ => {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("{} has no children", component.id),
            );
        }
    }
}

fn component_label(surface: &Surface, id: &str, base_path: Option<&str>) -> Option<String> {
    let component = surface.components.get(id)?;
    match component.kind.as_str() {
        "Text" => resolve_string(surface, component.props.get("text"), base_path)
            .map(|s| strip_markdown_heading(&s)),
        "Icon" => resolve_string(surface, component.props.get("name"), base_path)
            .map(|name| format!("[{name}]")),
        "Button" => string_prop(&component.props, "child")
            .and_then(|child| component_label(surface, child, base_path)),
        _ => Some(component.kind.clone()),
    }
}

fn dispatch_action(surface: &Surface, action: &Value, base_path: Option<&str>) {
    if let Some(event) = action.get("event").and_then(Value::as_object) {
        let name = string_prop(event, "name").unwrap_or("unnamed");
        let context = event
            .get("context")
            .map(|ctx| resolve_action_context(surface, ctx, base_path))
            .unwrap_or(Value::Null);
        println!("a2ui-egui-renderer: action event={name} context={context}");
    } else if let Some(call) = action.get("functionCall") {
        let result = eval_function(surface, call, base_path);
        println!("a2ui-egui-renderer: action function result={result}");
    } else {
        println!("a2ui-egui-renderer: action {action}");
    }
}

fn resolve_action_context(surface: &Surface, context: &Value, base_path: Option<&str>) -> Value {
    match context {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), resolve_value(surface, Some(value), base_path)))
                .collect(),
        ),
        _ => resolve_value(surface, Some(context), base_path),
    }
}

fn check_errors(
    surface: &Surface,
    props: &Map<String, Value>,
    base_path: Option<&str>,
) -> Vec<String> {
    props
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|check| {
            let condition = check.get("condition");
            let ok = resolve_bool(surface, condition, base_path).unwrap_or(true);
            if ok {
                None
            } else {
                Some(
                    check
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Validation failed.")
                        .to_owned(),
                )
            }
        })
        .collect()
}

fn read_value_or_local(
    surface: &Surface,
    component_id: &str,
    expr: Option<&Value>,
    base_path: Option<&str>,
) -> Value {
    let resolved = resolve_value(surface, expr, base_path);
    if !resolved.is_null() {
        resolved
    } else {
        surface
            .local_values
            .get(component_id)
            .cloned()
            .unwrap_or(Value::Null)
    }
}

fn write_value_or_local(
    surface: &mut Surface,
    component_id: &str,
    expr: Option<&Value>,
    value: Value,
    base_path: Option<&str>,
) {
    if let Some(path) = expr.and_then(extract_path) {
        let path = resolve_path(path, base_path);
        set_json_path(&mut surface.data_model, &path, value);
    } else {
        surface.local_values.insert(component_id.to_owned(), value);
    }
}

fn resolve_value(surface: &Surface, expr: Option<&Value>, base_path: Option<&str>) -> Value {
    let Some(expr) = expr else {
        return Value::Null;
    };
    match expr {
        Value::Object(map) => {
            if let Some(path) = map.get("path").and_then(Value::as_str) {
                let path = resolve_path(path, base_path);
                surface
                    .data_model
                    .pointer(&path)
                    .cloned()
                    .unwrap_or(Value::Null)
            } else if let Some(v) = map.get("literalString") {
                v.clone()
            } else if let Some(v) = map.get("literalNumber") {
                v.clone()
            } else if let Some(v) = map.get("literalBoolean") {
                v.clone()
            } else if let Some(v) = map.get("literalArray") {
                v.clone()
            } else if map.contains_key("call") {
                eval_function(surface, expr, base_path)
            } else {
                expr.clone()
            }
        }
        _ => expr.clone(),
    }
}

fn resolve_string(
    surface: &Surface,
    expr: Option<&Value>,
    base_path: Option<&str>,
) -> Option<String> {
    match resolve_value(surface, expr, base_path) {
        Value::Null => None,
        Value::String(s) => Some(s),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Array(a) => Some(Value::Array(a).to_string()),
        Value::Object(o) => Some(Value::Object(o).to_string()),
    }
}

fn resolve_number(surface: &Surface, expr: Option<&Value>, base_path: Option<&str>) -> Option<f64> {
    match resolve_value(surface, expr, base_path) {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        Value::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn resolve_bool(surface: &Surface, expr: Option<&Value>, base_path: Option<&str>) -> Option<bool> {
    match resolve_value(surface, expr, base_path) {
        Value::Bool(b) => Some(b),
        Value::String(s) => Some(!s.is_empty()),
        Value::Number(n) => Some(n.as_f64().unwrap_or(0.0) != 0.0),
        Value::Array(a) => Some(!a.is_empty()),
        Value::Object(o) => Some(!o.is_empty()),
        Value::Null => None,
    }
}

fn resolve_string_list(
    surface: &Surface,
    expr: Option<&Value>,
    base_path: Option<&str>,
) -> Option<Vec<String>> {
    match resolve_value(surface, expr, base_path) {
        Value::Array(items) => Some(
            items
                .into_iter()
                .map(|item| match item {
                    Value::String(s) => s,
                    other => other.to_string(),
                })
                .collect(),
        ),
        Value::String(s) => Some(vec![s]),
        Value::Null => None,
        other => Some(vec![other.to_string()]),
    }
}

fn eval_function(surface: &Surface, expr: &Value, base_path: Option<&str>) -> Value {
    let Some(map) = expr.as_object() else {
        return Value::Null;
    };
    let Some(call) = map.get("call").and_then(Value::as_str) else {
        return Value::Null;
    };
    let args = map.get("args").and_then(Value::as_object);

    match call {
        "required" => {
            Value::Bool(args.and_then(|a| a.get("value")).is_some_and(|value| {
                !is_empty_value(&resolve_value(surface, Some(value), base_path))
            }))
        }
        "regex" => {
            let value = args
                .and_then(|a| a.get("value"))
                .and_then(|v| resolve_string(surface, Some(v), base_path))
                .unwrap_or_default();
            let pattern = args
                .and_then(|a| a.get("pattern"))
                .and_then(Value::as_str)
                .unwrap_or("");
            Value::Bool(
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(&value))
                    .unwrap_or(false),
            )
        }
        "length" => {
            let value = args
                .and_then(|a| a.get("value"))
                .and_then(|v| resolve_string(surface, Some(v), base_path))
                .unwrap_or_default();
            let min = args.and_then(|a| a.get("min")).and_then(Value::as_u64);
            let max = args.and_then(|a| a.get("max")).and_then(Value::as_u64);
            let len = value.chars().count() as u64;
            Value::Bool(min.map_or(true, |m| len >= m) && max.map_or(true, |m| len <= m))
        }
        "numeric" => {
            let value = args
                .and_then(|a| a.get("value"))
                .and_then(|v| resolve_number(surface, Some(v), base_path))
                .unwrap_or(0.0);
            let min = args.and_then(|a| a.get("min")).and_then(Value::as_f64);
            let max = args.and_then(|a| a.get("max")).and_then(Value::as_f64);
            Value::Bool(min.map_or(true, |m| value >= m) && max.map_or(true, |m| value <= m))
        }
        "email" => {
            let value = args
                .and_then(|a| a.get("value"))
                .and_then(|v| resolve_string(surface, Some(v), base_path))
                .unwrap_or_default();
            Value::Bool(value.contains('@') && value.rsplit_once('.').is_some())
        }
        "formatString" => args
            .and_then(|a| a.get("value"))
            .and_then(|v| resolve_string(surface, Some(v), base_path))
            .map(Value::String)
            .unwrap_or(Value::Null),
        "formatNumber" => {
            let value = args
                .and_then(|a| a.get("value"))
                .and_then(|v| resolve_number(surface, Some(v), base_path))
                .unwrap_or(0.0);
            let decimals = args
                .and_then(|a| a.get("decimals"))
                .and_then(|v| resolve_number(surface, Some(v), base_path))
                .unwrap_or(0.0) as usize;
            Value::String(format!("{value:.decimals$}"))
        }
        "formatCurrency" => {
            let value = args
                .and_then(|a| a.get("value"))
                .and_then(|v| resolve_number(surface, Some(v), base_path))
                .unwrap_or(0.0);
            let currency = args
                .and_then(|a| a.get("currency"))
                .and_then(|v| resolve_string(surface, Some(v), base_path))
                .unwrap_or_else(|| "USD".to_owned());
            Value::String(format!("{currency} {value:.2}"))
        }
        "formatDate" => args
            .and_then(|a| a.get("value"))
            .and_then(|v| resolve_string(surface, Some(v), base_path))
            .map(Value::String)
            .unwrap_or(Value::Null),
        "pluralize" => {
            let value = args
                .and_then(|a| a.get("value"))
                .and_then(|v| resolve_number(surface, Some(v), base_path))
                .unwrap_or(0.0);
            let key = if value == 0.0 {
                "zero"
            } else if value == 1.0 {
                "one"
            } else {
                "other"
            };
            let fallback = args.and_then(|a| a.get("other"));
            args.and_then(|a| a.get(key))
                .or(fallback)
                .and_then(|v| resolve_string(surface, Some(v), base_path))
                .map(Value::String)
                .unwrap_or(Value::Null)
        }
        "and" => Value::Bool(
            args.and_then(|a| a.get("values"))
                .and_then(Value::as_array)
                .is_some_and(|values| {
                    values
                        .iter()
                        .all(|value| resolve_bool(surface, Some(value), base_path).unwrap_or(false))
                }),
        ),
        "or" => Value::Bool(
            args.and_then(|a| a.get("values"))
                .and_then(Value::as_array)
                .is_some_and(|values| {
                    values
                        .iter()
                        .any(|value| resolve_bool(surface, Some(value), base_path).unwrap_or(false))
                }),
        ),
        "not" => Value::Bool(
            !args
                .and_then(|a| a.get("value"))
                .and_then(|v| resolve_bool(surface, Some(v), base_path))
                .unwrap_or(false),
        ),
        "openUrl" => {
            if let Some(url) = args.and_then(|a| a.get("url")).and_then(Value::as_str) {
                println!("a2ui-egui-renderer: openUrl requested: {url}");
            }
            Value::Null
        }
        _ => Value::Null,
    }
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

fn extract_path(expr: &Value) -> Option<&str> {
    expr.as_object()
        .and_then(|obj| obj.get("path"))
        .and_then(Value::as_str)
}

fn resolve_path(path: &str, base_path: Option<&str>) -> String {
    if path.starts_with('/') {
        path.to_owned()
    } else if let Some(base) = base_path {
        if base.ends_with('/') {
            format!("{base}{path}")
        } else {
            format!("{base}/{path}")
        }
    } else {
        format!("/{path}")
    }
}

fn template_base_paths(surface: &Surface, path: &str, base_path: Option<&str>) -> Vec<String> {
    let path = resolve_path(path, base_path);
    match surface.data_model.pointer(&path) {
        Some(Value::Array(items)) => (0..items.len())
            .map(|idx| format!("{path}/{idx}"))
            .collect(),
        Some(Value::Object(map)) => map
            .keys()
            .map(|key| format!("{path}/{}", escape_json_pointer(key)))
            .collect(),
        _ => Vec::new(),
    }
}

fn set_json_path(root: &mut Value, path: &str, value: Value) {
    if path.is_empty() || path == "/" {
        *root = value;
        return;
    }

    let parts = json_pointer_parts(path);
    if parts.is_empty() {
        *root = value;
        return;
    }

    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        if !current.is_object() && !current.is_array() {
            *current = Value::Object(Map::new());
        }
        match current {
            Value::Object(map) => {
                current = map
                    .entry(part.clone())
                    .or_insert_with(|| Value::Object(Map::new()));
            }
            Value::Array(items) => {
                let idx = part.parse::<usize>().unwrap_or(items.len());
                while items.len() <= idx {
                    items.push(Value::Object(Map::new()));
                }
                current = &mut items[idx];
            }
            _ => unreachable!(),
        }
    }

    let last = parts.last().expect("parts is not empty").clone();
    match current {
        Value::Object(map) => {
            map.insert(last, value);
        }
        Value::Array(items) => {
            let idx = last.parse::<usize>().unwrap_or(items.len());
            while items.len() <= idx {
                items.push(Value::Null);
            }
            items[idx] = value;
        }
        _ => {
            let mut map = Map::new();
            map.insert(last, value);
            *current = Value::Object(map);
        }
    }
}

fn remove_json_path(root: &mut Value, path: &str) {
    if path.is_empty() || path == "/" {
        *root = Value::Null;
        return;
    }
    let parts = json_pointer_parts(path);
    let Some((last, parents)) = parts.split_last() else {
        return;
    };
    let mut current = root;
    for part in parents {
        match current {
            Value::Object(map) => {
                let Some(next) = map.get_mut(part) else {
                    return;
                };
                current = next;
            }
            Value::Array(items) => {
                let Some(next) = part
                    .parse::<usize>()
                    .ok()
                    .and_then(|idx| items.get_mut(idx))
                else {
                    return;
                };
                current = next;
            }
            _ => return,
        }
    }
    match current {
        Value::Object(map) => {
            map.remove(last);
        }
        Value::Array(items) => {
            if let Ok(idx) = last.parse::<usize>() {
                if let Some(slot) = items.get_mut(idx) {
                    *slot = Value::Null;
                }
            }
        }
        _ => {}
    }
}

fn json_pointer_parts(path: &str) -> Vec<String> {
    path.trim_start_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| part.replace("~1", "/").replace("~0", "~"))
        .collect()
}

fn escape_json_pointer(part: &str) -> String {
    part.replace('~', "~0").replace('/', "~1")
}

fn parse_messages(input: &str) -> Result<Vec<Value>, BoxError> {
    let trimmed = strip_code_fence(input.trim());
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(match value {
            Value::Array(items) => items,
            Value::Object(mut object) => {
                if let Some(Value::Array(messages)) = object.remove("messages") {
                    messages
                } else {
                    vec![Value::Object(object)]
                }
            }
            other => vec![other],
        });
    }

    let mut messages = Vec::new();
    for (idx, line) in trimmed.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value =
            serde_json::from_str(line).map_err(|e| format!("line {}: {e}", idx + 1))?;
        messages.push(value);
    }
    Ok(messages)
}

fn strip_code_fence(input: &str) -> &str {
    let input = input.trim();
    if !input.starts_with("```") {
        return input;
    }
    let Some(first_newline) = input.find('\n') else {
        return input;
    };
    let body = &input[first_newline + 1..];
    body.strip_suffix("```").unwrap_or(body).trim()
}

fn data_contents_to_value(contents: &[Value]) -> Value {
    let mut map = Map::new();
    for entry in contents.iter().filter_map(Value::as_object) {
        if let Some(key) = string_prop(entry, "key") {
            map.insert(key.to_owned(), data_entry_to_value(entry));
        }
    }
    Value::Object(map)
}

fn data_entry_to_value(entry: &Map<String, Value>) -> Value {
    if let Some(value) = entry.get("valueString") {
        value.clone()
    } else if let Some(value) = entry.get("valueNumber") {
        value.clone()
    } else if let Some(value) = entry.get("valueBoolean") {
        value.clone()
    } else if let Some(items) = entry.get("valueMap").and_then(Value::as_array) {
        data_contents_to_value(items)
    } else {
        Value::Null
    }
}

fn string_prop<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str)
}

fn strip_markdown_heading(text: &str) -> String {
    text.trim_start_matches('#').trim_start().to_owned()
}

fn component_identity(id: &str, base_path: Option<&str>) -> String {
    match base_path {
        Some(path) => format!("{id}@{path}"),
        None => id.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_jsonl_and_builds_surface() {
        let messages = parse_messages(BUILTIN_SAMPLE).unwrap();
        let mut runtime = A2uiRuntime::default();
        runtime.process_messages(messages).unwrap();
        let surface = runtime.surfaces.get("contact_form_1").unwrap();
        assert_eq!(surface.root, "root");
        assert!(surface.components.contains_key("submit_button"));
        assert_eq!(
            surface.data_model.pointer("/contact/email"),
            Some(&json!("john.doe@example.com"))
        );
    }

    #[test]
    fn resolves_v08_component_wrapper() {
        let raw = json!({
            "id": "title",
            "component": {
                "Text": {
                    "text": {"literalString": "Hello"},
                    "usageHint": "h1"
                }
            }
        });
        let component = Component::from_value(&raw).unwrap();
        assert_eq!(component.kind, "Text");
        assert_eq!(component.props.get("usageHint"), Some(&json!("h1")));
    }

    #[test]
    fn writes_nested_json_pointer() {
        let mut value = Value::Null;
        set_json_path(&mut value, "/contact/name", json!("Ada"));
        assert_eq!(value.pointer("/contact/name"), Some(&json!("Ada")));
    }
}
