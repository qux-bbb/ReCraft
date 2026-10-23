// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ReCraft desktop entry point.
//!
//! Rendering backend: **Glow (OpenGL)**. eframe's default wgpu/DX12 path crashed at
//! startup on an Intel UHD driver during the UI spike (see `docs/adr/ADR-002`), so the
//! product pins Glow until a renderer fallback is implemented.

mod app;
mod i18n;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let initial = std::env::args().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ReCraft")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ReCraft",
        options,
        Box::new(move |cc| Ok(Box::new(app::ReCraftApp::new(cc, initial)))),
    )
}
