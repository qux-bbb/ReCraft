// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Technical spike: can ReCraft render a detached pane in a real OS window with the
//! pinned Glow renderer? Run with:
//!
//! ```powershell
//! cargo run -p recraft-ui --example viewport_spike
//! ```
//!
//! The UI reports how egui classified the second viewport (`Immediate` = a real OS
//! window, `EmbeddedWindow` = backend fallback). See `docs/adr/ADR-012-multi-window.md`.

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("ReCraft viewport spike - main")
            .with_inner_size([640.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "viewport-spike",
        options,
        Box::new(|_cc| Ok(Box::new(Spike::default()))),
    )
}

fn class_name(class: egui::ViewportClass) -> &'static str {
    match class {
        egui::ViewportClass::Root => "Root",
        egui::ViewportClass::Deferred => "Deferred",
        egui::ViewportClass::Immediate => "Immediate",
        egui::ViewportClass::EmbeddedWindow => "EmbeddedWindow",
    }
}

struct Spike {
    open: bool,
    clicks: u32,
    last_class: Option<String>,
}

impl Default for Spike {
    fn default() -> Self {
        Self {
            // Keep the detached viewport open so the spike is visible immediately.
            open: true,
            clicks: 0,
            last_class: None,
        }
    }
}

impl eframe::App for Spike {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ui.heading("ReCraft multi-viewport spike");
        ui.label("This window is the main viewport.");
        ui.checkbox(&mut self.open, "Open detached pane in a real OS window");
        if let Some(class) = self.last_class.as_deref() {
            ui.monospace(format!("second viewport class: {class}"));
        }

        if !self.open {
            return;
        }
        let viewport_id = egui::ViewportId::from_hash_of("recraft-detached-spike");
        let builder = egui::ViewportBuilder::default()
            .with_title("ReCraft detached pane (spike)")
            .with_inner_size([520.0, 320.0])
            .with_min_inner_size([320.0, 200.0]);
        let mut close = false;
        let clicks = &mut self.clicks;
        let last_class = &mut self.last_class;
        let class = ctx.show_viewport_immediate(viewport_id, builder, |ui, class| {
            *last_class = Some(class_name(class).to_owned());
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading("Detached pane");
                ui.label("If this is a separate OS window, docking can be promoted later.");
                if ui.button(format!("Clicks: {clicks}")).clicked() {
                    *clicks += 1;
                }
                if ui.input(|input| input.viewport().close_requested()) {
                    close = true;
                }
            });
            class
        });
        self.last_class = Some(class_name(class).to_owned());
        if close {
            self.open = false;
        }
    }
}
