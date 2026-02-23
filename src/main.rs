mod agent;
mod app;
mod config;
mod export;
mod simulation;

use app::App;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Prisoner's Dilemma Simulation"),
        ..Default::default()
    };

    eframe::run_native(
        "Prisoner's Dilemma Simulation",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
