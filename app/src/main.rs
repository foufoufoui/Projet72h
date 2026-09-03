#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod config;
mod menus;
mod model;
mod pipette;
mod theme;

use eframe::egui;

/// Point d'entrée : configure la fenêtre (taille, titre) puis lance la boucle
/// eframe avec l'application `HarpeApp`.
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([860.0, 540.0])
            .with_title("Harpe sans corde"),
        ..Default::default()
    };
    eframe::run_native(
        "Harpe sans corde",
        options,
        Box::new(|cc| Ok(Box::new(app::HarpeApp::new(cc)))),
    )
}
