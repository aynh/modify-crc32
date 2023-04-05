use app::App;
use worker::Worker;

mod app;
mod worker;

fn main() -> Result<(), eframe::Error> {
    let (tx, rx) = Worker::spawn("modify-crc32-worker".to_string());
    eframe::run_native(
        "modify-crc32",
        eframe::NativeOptions {
            initial_window_size: Some([280.0, 240.0].into()),
            resizable: false,
            ..Default::default()
        },
        Box::new(|_cc| Box::new(App::new(tx, rx))),
    )
}
