use eframe::{egui, WindowType};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        title: "egui Chat Overlay Demo".to_owned(),
        app_id: "rust.tizen.chat-overlay".to_owned(),
        size: (520, 900),
        continuous_repaint: true,
        // Floating + transparent = partial overlay that sits over the
        // launcher/toplevel below, blending RGBA pixels through.
        window_type: WindowType::Floating,
        transparent: true,
        // Passive overlay: leave focus on the launcher so its remote
        // navigation keeps working. `close_on_back` still wires Back
        // to exit — picking Exclusive grab mode automatically so the
        // overlay receives Back even without focus.
        focus_skip: true,
        ..Default::default()
    };
    eframe::run_native(
        "egui Chat Overlay Demo",
        options,
        Box::new(|cc| Ok(Box::new(ChatApp::new(cc)))),
    )
}

#[derive(Clone)]
enum MessageKind {
    Text { author: String, body: String, accent: egui::Color32 },
    System { body: String },
    Code { author: String, lang: String, code: String },
    Alert { level: AlertLevel, body: String },
    Reaction { author: String, emoji: String, target: String },
}

#[derive(Clone, Copy)]
enum AlertLevel { Info, Warn, Error }

impl AlertLevel {
    fn color(self) -> egui::Color32 {
        match self {
            AlertLevel::Info => egui::Color32::from_rgb(96, 165, 250),
            AlertLevel::Warn => egui::Color32::from_rgb(251, 191, 36),
            AlertLevel::Error => egui::Color32::from_rgb(248, 113, 113),
        }
    }
    fn icon(self) -> &'static str {
        match self { AlertLevel::Info => "ℹ", AlertLevel::Warn => "⚠", AlertLevel::Error => "✕" }
    }
}

struct ChatMessage {
    kind: MessageKind,
    born: Instant,
}

struct ChatApp {
    messages: VecDeque<ChatMessage>,
    last_spawn: Instant,
    spawn_interval: Duration,
    script_idx: usize,
}

impl ChatApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        use egui::{FontFamily, FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(34.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(22.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(20.0, FontFamily::Monospace)),
            (TextStyle::Button, FontId::new(22.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(16.0, FontFamily::Proportional)),
        ]
        .into();
        cc.egui_ctx.set_style(style);

        Self {
            messages: VecDeque::new(),
            last_spawn: Instant::now(),
            spawn_interval: Duration::from_millis(1400),
            script_idx: 0,
        }
    }

    fn maybe_spawn(&mut self) {
        if self.last_spawn.elapsed() < self.spawn_interval {
            return;
        }
        self.last_spawn = Instant::now();

        let palette = [
            egui::Color32::from_rgb(244, 114, 182), // pink
            egui::Color32::from_rgb(167, 139, 250), // violet
            egui::Color32::from_rgb(52, 211, 153),  // emerald
            egui::Color32::from_rgb(251, 191, 36),  // amber
            egui::Color32::from_rgb(96, 165, 250),  // blue
        ];

        let script: Vec<MessageKind> = vec![
            MessageKind::System { body: "— demo session started —".into() },
            MessageKind::Text { author: "ada".into(), body: "egui on a TV. surprisingly smooth.".into(), accent: palette[0] },
            MessageKind::Text { author: "linus".into(), body: "binary is under 4 MB stripped 🤯".into(), accent: palette[1] },
            MessageKind::Reaction { author: "grace".into(), emoji: "🔥".into(), target: "linus".into() },
            MessageKind::Code { author: "ada".into(), lang: "rust".into(), code: "ui.label(\"hello tizen\");".into() },
            MessageKind::Alert { level: AlertLevel::Info, body: "frame time: 3.2ms".into() },
            MessageKind::Text { author: "turing".into(), body: "immediate mode means the UI is the data.".into(), accent: palette[2] },
            MessageKind::Alert { level: AlertLevel::Warn, body: "GPU memory 62%".into() },
            MessageKind::Text { author: "hopper".into(), body: "no XML, no CSS, no DOM. just a loop.".into(), accent: palette[3] },
            MessageKind::Reaction { author: "ada".into(), emoji: "👏".into(), target: "hopper".into() },
            MessageKind::Code { author: "linus".into(), lang: "rust".into(), code: "for m in &msgs { render(ui, m); }".into() },
            MessageKind::Alert { level: AlertLevel::Error, body: "simulated: socket dropped".into() },
            MessageKind::System { body: "reconnecting…".into() },
            MessageKind::Text { author: "grace".into(), body: "back online. that was fast.".into(), accent: palette[4] },
        ];

        let msg = script[self.script_idx % script.len()].clone();
        self.script_idx += 1;
        self.messages.push_back(ChatMessage { kind: msg, born: Instant::now() });
        if self.messages.len() > 40 {
            self.messages.pop_front();
        }
    }
}

impl eframe::App for ChatApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.maybe_spawn();
        ctx.request_repaint_after(Duration::from_millis(33));

        let panel_frame = egui::Frame {
            fill: egui::Color32::from_rgba_premultiplied(12, 14, 22, 210),
            inner_margin: egui::Margin::symmetric(20.0, 18.0),
            rounding: egui::Rounding::same(0.0),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 18)),
            ..Default::default()
        };

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 6.0, egui::Color32::from_rgb(52, 211, 153));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("LIVE CHAT").size(24.0).strong().color(egui::Color32::WHITE));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{} msgs", self.messages.len()))
                                .size(16.0)
                                .color(egui::Color32::from_gray(160)),
                        );
                    });
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for msg in &self.messages {
                            render_message(ui, msg);
                            ui.add_space(10.0);
                        }
                    });
            });
    }
}

fn render_message(ui: &mut egui::Ui, msg: &ChatMessage) {
    let age = msg.born.elapsed().as_secs_f32();
    let alpha = (age / 0.35).clamp(0.0, 1.0);
    let offset_x = (1.0 - alpha) * 20.0;

    let tint = |c: egui::Color32, a: f32| -> egui::Color32 {
        let aa = (c.a() as f32 * a) as u8;
        egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), aa)
    };

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.add_space(offset_x);

        match &msg.kind {
            MessageKind::Text { author, body, accent } => {
                egui::Frame::default()
                    .fill(tint(egui::Color32::from_rgba_premultiplied(255, 255, 255, 10), alpha))
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .rounding(egui::Rounding::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 5.0, tint(*accent, alpha));
                            ui.label(egui::RichText::new(author).size(20.0).color(tint(*accent, alpha)).strong());
                        });
                        ui.label(egui::RichText::new(body).size(22.0).color(tint(egui::Color32::from_gray(235), alpha)));
                    });
            }
            MessageKind::System { body } => {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(body)
                            .size(16.0)
                            .italics()
                            .color(tint(egui::Color32::from_gray(140), alpha)),
                    );
                });
            }
            MessageKind::Code { author, lang, code } => {
                egui::Frame::default()
                    .fill(tint(egui::Color32::from_rgba_premultiplied(0, 0, 0, 140), alpha))
                    .inner_margin(egui::Margin::same(12.0))
                    .rounding(egui::Rounding::same(10.0))
                    .stroke(egui::Stroke::new(1.0, tint(egui::Color32::from_rgb(99, 102, 241), alpha)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(author)
                                    .size(18.0)
                                    .strong()
                                    .color(tint(egui::Color32::from_rgb(165, 180, 252), alpha)),
                            );
                            ui.label(
                                egui::RichText::new(format!("· {}", lang))
                                    .size(14.0)
                                    .color(tint(egui::Color32::from_gray(140), alpha)),
                            );
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(code)
                                .monospace()
                                .color(tint(egui::Color32::from_rgb(186, 230, 253), alpha)),
                        );
                    });
            }
            MessageKind::Alert { level, body } => {
                let c = level.color();
                egui::Frame::default()
                    .fill(tint(egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 35), alpha))
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .rounding(egui::Rounding::same(10.0))
                    .stroke(egui::Stroke::new(1.0, tint(c, alpha * 0.6)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(level.icon()).size(22.0).color(tint(c, alpha)));
                            ui.label(
                                egui::RichText::new(body)
                                    .size(20.0)
                                    .color(tint(egui::Color32::from_gray(240), alpha)),
                            );
                        });
                    });
            }
            MessageKind::Reaction { author, emoji, target } => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(emoji).size(28.0));
                    ui.label(
                        egui::RichText::new(format!("{} reacted to {}", author, target))
                            .size(16.0)
                            .italics()
                            .color(tint(egui::Color32::from_gray(170), alpha)),
                    );
                });
            }
        }
    });
}
