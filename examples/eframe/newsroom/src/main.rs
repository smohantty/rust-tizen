//! Newsroom — a streaming-text overlay demoing the eframe ↔ tokio
//! bidirectional pattern.
//!
//! Architecture:
//!
//! ```text
//!  Main thread (calloop UI loop)        Worker thread (tokio runtime)
//!  ─────────────────────────────        ──────────────────────────────
//!  App::update {                         dispatcher task {
//!    drain Event channel  ◄────────────  for each token sleep+send
//!    render form          ─Cmd────────►  match cmd { spawn / cancel }
//!    request_repaint                     ctx.request_repaint after send
//!  }
//! ```
//!
//! - User picks a category, types a seed, clicks **Generate** →
//!   `Cmd::Generate` over the mpsc channel.
//! - Backend spawns a tokio task that walks a canned word list, sends
//!   one `Event::Token` per ~120ms.
//! - Cancel button → `Cmd::Cancel(id)` → backend trips the task's
//!   `oneshot::Receiver` and the task bails out mid-loop.
//! - UI calls `egui::Context::request_repaint` from the worker side so
//!   tokens appear within one frame.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use eframe::{egui, keys, App, CreationContext, Frame, KeyGrab, NativeOptions, WindowType};
use tokio::sync::{mpsc, oneshot};

// ----- shared types --------------------------------------------------------

type GenId = u64;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Category {
    Headline,
    StockBlurb,
    WeatherPoem,
    Fortune,
}

impl Category {
    fn label(self) -> &'static str {
        match self {
            Self::Headline => "Headline",
            Self::StockBlurb => "Stock blurb",
            Self::WeatherPoem => "Weather poem",
            Self::Fortune => "Fortune",
        }
    }

    fn corpus(self, seed: &str) -> Vec<&'static str> {
        // Tiny canned bag of words per category. A real backend would
        // call an LLM API; this lets us run offline and still demo
        // the streaming pattern faithfully.
        let _ = seed; // seed is rendered in UI; not used by the mock corpus
        match self {
            Self::Headline => vec![
                "Local",
                "rust",
                "wayland",
                "compositor",
                "ships",
                "tearing-free",
                "overlay",
                "support",
                ";",
                "users",
                "rejoice",
                ".",
            ],
            Self::StockBlurb => vec![
                "$RUST",
                "up",
                "3.4%",
                "on",
                "stronger",
                "tokio",
                "adoption",
                ";",
                "analysts",
                "see",
                "more",
                "upside",
                "into",
                "Q3",
                ".",
            ],
            Self::WeatherPoem => vec![
                "soft",
                "wind",
                "over",
                "yongin",
                ",",
                "the",
                "compositor",
                "hums",
                ",",
                "a",
                "single",
                "frame",
                "shimmers",
                "by",
                ".",
            ],
            Self::Fortune => vec![
                "A",
                "long-awaited",
                "PR",
                "will",
                "merge",
                "before",
                "Friday",
                ".",
                "Avoid",
                "force-pushes",
                "to",
                "main",
                ".",
            ],
        }
    }
}

enum Cmd {
    Generate {
        category: Category,
        seed: String,
        id: GenId,
    },
    Cancel(GenId),
}

#[derive(Debug)]
enum Event {
    Started(GenId),
    Token(GenId, String),
    Finished(GenId),
}

// ----- app state -----------------------------------------------------------

struct InFlight {
    id: GenId,
    category: Category,
    text: String,
    started: Instant,
}

struct Snippet {
    category: Category,
    text: String,
}

struct Newsroom {
    cmd_tx: mpsc::UnboundedSender<Cmd>,
    event_rx: mpsc::UnboundedReceiver<Event>,
    next_id: GenId,

    category: Category,
    seed: String,
    in_flight: Option<InFlight>,
    history: VecDeque<Snippet>,
}

impl Newsroom {
    fn new(
        cc: &CreationContext,
        cmd_tx: mpsc::UnboundedSender<Cmd>,
        event_rx: mpsc::UnboundedReceiver<Event>,
    ) -> Self {
        // Bump default font sizes for TV viewing distance.
        let mut style = (*cc.egui_ctx.style()).clone();
        use egui::{FontFamily, FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(30.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(18.0, FontFamily::Monospace)),
            (TextStyle::Button, FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(15.0, FontFamily::Proportional)),
        ]
        .into();
        cc.egui_ctx.set_style(style);

        Self {
            cmd_tx,
            event_rx,
            next_id: 1,
            category: Category::Headline,
            seed: "tizen".to_owned(),
            in_flight: None,
            history: VecDeque::new(),
        }
    }

    fn apply_event(&mut self, ev: Event) {
        match ev {
            Event::Started(id) => {
                if let Some(f) = &mut self.in_flight {
                    if f.id == id {
                        f.started = Instant::now();
                    }
                }
            }
            Event::Token(id, chunk) => {
                if let Some(f) = &mut self.in_flight {
                    if f.id == id {
                        if !f.text.is_empty() && !chunk.starts_with(['.', ',', ';', '!', '?']) {
                            f.text.push(' ');
                        }
                        f.text.push_str(&chunk);
                    }
                }
            }
            Event::Finished(id) => {
                if let Some(f) = self.in_flight.take_if(|f| f.id == id) {
                    self.history.push_front(Snippet {
                        category: f.category,
                        text: f.text,
                    });
                    if self.history.len() > 8 {
                        self.history.pop_back();
                    }
                }
            }
        }
    }

    fn submit(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        self.in_flight = Some(InFlight {
            id,
            category: self.category,
            text: String::new(),
            started: Instant::now(),
        });
        let _ = self.cmd_tx.send(Cmd::Generate {
            category: self.category,
            seed: self.seed.clone(),
            id,
        });
    }

    fn cancel(&mut self) {
        if let Some(f) = &self.in_flight {
            let _ = self.cmd_tx.send(Cmd::Cancel(f.id));
        }
    }
}

impl App for Newsroom {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // 1. drain backend events
        while let Ok(ev) = self.event_rx.try_recv() {
            self.apply_event(ev);
        }

        // 2. ensure the overlay keeps animating the cursor while we're
        //    waiting for the next token even if no other event lands.
        if self.in_flight.is_some() {
            ctx.request_repaint_after(Duration::from_millis(120));
        }

        let panel_frame = egui::Frame {
            fill: egui::Color32::from_rgba_premultiplied(12, 14, 22, 220),
            inner_margin: egui::Margin::same(18.0),
            rounding: egui::Rounding::same(0.0),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 24)),
            ..Default::default()
        };

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 5.0, egui::Color32::from_rgb(52, 211, 153));
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("NEWSROOM").size(22.0).strong().color(egui::Color32::WHITE));
                });
                ui.separator();

                // Category selector
                ui.horizontal(|ui| {
                    ui.label("Category:");
                    egui::ComboBox::from_id_salt("cat")
                        .selected_text(self.category.label())
                        .show_ui(ui, |ui| {
                            for cat in [Category::Headline, Category::StockBlurb, Category::WeatherPoem, Category::Fortune] {
                                ui.selectable_value(&mut self.category, cat, cat.label());
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Seed:");
                    ui.text_edit_singleline(&mut self.seed);
                });

                ui.add_space(6.0);
                let generating = self.in_flight.is_some();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!generating, egui::Button::new("Generate"))
                        .clicked()
                    {
                        self.submit();
                    }
                    if ui
                        .add_enabled(generating, egui::Button::new("Cancel"))
                        .clicked()
                    {
                        self.cancel();
                    }
                    if let Some(f) = &self.in_flight {
                        ui.label(
                            egui::RichText::new(format!("streaming ({:.1}s)", f.started.elapsed().as_secs_f32()))
                                .color(egui::Color32::from_gray(170)),
                        );
                    }
                });

                ui.add_space(10.0);
                ui.separator();

                // In-flight stream
                if let Some(f) = &self.in_flight {
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgba_premultiplied(255, 255, 255, 12))
                        .inner_margin(egui::Margin::same(12.0))
                        .rounding(egui::Rounding::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(f.category.label())
                                    .size(16.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(167, 139, 250)),
                            );
                            // Blinking cursor: even half-second on
                            let blink_on = (f.started.elapsed().as_millis() / 400) % 2 == 0;
                            let cursor = if blink_on { "▌" } else { " " };
                            ui.label(
                                egui::RichText::new(format!("{}{}", f.text, cursor))
                                    .size(22.0)
                                    .color(egui::Color32::from_gray(240)),
                            );
                        });
                    ui.add_space(10.0);
                }

                // History feed
                ui.label(egui::RichText::new("History").color(egui::Color32::from_gray(160)));
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for snip in &self.history {
                        egui::Frame::default()
                            .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 90))
                            .inner_margin(egui::Margin::same(10.0))
                            .rounding(egui::Rounding::same(8.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(snip.category.label())
                                        .size(14.0)
                                        .color(egui::Color32::from_rgb(96, 165, 250)),
                                );
                                ui.label(
                                    egui::RichText::new(&snip.text)
                                        .size(18.0)
                                        .color(egui::Color32::from_gray(220)),
                                );
                            });
                        ui.add_space(6.0);
                    }
                });
            });
    }
}

// ----- backend -------------------------------------------------------------

async fn run_backend(
    mut cmd_rx: mpsc::UnboundedReceiver<Cmd>,
    event_tx: mpsc::UnboundedSender<Event>,
    ctx: egui::Context,
) {
    let mut active: HashMap<GenId, oneshot::Sender<()>> = HashMap::new();
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            Cmd::Generate { category, seed, id } => {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                active.insert(id, cancel_tx);
                let event_tx = event_tx.clone();
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    stream(id, category, seed, event_tx, ctx, cancel_rx).await;
                });
            }
            Cmd::Cancel(id) => {
                if let Some(tx) = active.remove(&id) {
                    let _ = tx.send(());
                }
            }
        }
    }
}

async fn stream(
    id: GenId,
    category: Category,
    seed: String,
    tx: mpsc::UnboundedSender<Event>,
    ctx: egui::Context,
    mut cancel: oneshot::Receiver<()>,
) {
    let _ = tx.send(Event::Started(id));
    ctx.request_repaint();

    let words = category.corpus(&seed);
    // Tiny LCG for delay jitter. Seed it from the request id so the
    // demo is repeatable per run but feels organic.
    let mut state: u64 = id.wrapping_mul(0x9E3779B97F4A7C15);
    let mut rand_byte = || -> u8 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 56) as u8
    };

    for word in words {
        let jitter = rand_byte() as u64;
        let delay_ms = 100 + (jitter % 180);
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {
                if tx.send(Event::Token(id, word.to_string())).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
            _ = &mut cancel => {
                let _ = tx.send(Event::Finished(id));
                ctx.request_repaint();
                return;
            }
        }
    }

    let _ = tx.send(Event::Finished(id));
    ctx.request_repaint();
}

// ----- main ----------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Cmd>();
    let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();

    let options = NativeOptions {
        title: "Newsroom".to_owned(),
        app_id: "rust.tizen.newsroom".to_owned(),
        size: (640, 900),
        continuous_repaint: false,
        window_type: WindowType::Floating,
        transparent: true,
        // Take focus; grab the navigation keys via the keyrouter.
        // (`close_on_back` is `true` by default, so Back is grabbed
        //  + wired to set_should_close automatically.) Names are X11
        //  keysyms — tizen-eframe resolves them against the
        //  compositor's keymap at startup.
        grab_keys: vec![
            KeyGrab::topmost(keys::LEFT),
            KeyGrab::topmost(keys::RIGHT),
            KeyGrab::topmost(keys::UP),
            KeyGrab::topmost(keys::DOWN),
            KeyGrab::topmost(keys::ENTER),
        ],
        ..Default::default()
    };

    eframe::run_native(
        "Newsroom",
        options,
        Box::new(move |cc| {
            // Spawn the tokio runtime on a dedicated worker thread. We
            // intentionally don't use `#[tokio::main]` — that would
            // hijack `main` and block our calloop UI loop.
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .worker_threads(2)
                    .build()
                    .expect("tokio runtime");
                rt.block_on(run_backend(cmd_rx, event_tx, ctx));
            });

            Ok(Box::new(Newsroom::new(cc, cmd_tx, event_rx)))
        }),
    )
}
