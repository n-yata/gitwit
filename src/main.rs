mod app;
mod config;
mod git;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Gitwit")
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Gitwit",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
