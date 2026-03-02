mod app;
mod diagnostics;
mod tray;
mod ui;
mod wiring;

fn main() {
    app::controller::run();
}
