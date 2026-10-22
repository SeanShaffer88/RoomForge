// Release builds are a desktop app: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use std::io::Write;

fn log_path() -> std::path::PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if !local.is_empty() {
            return std::path::PathBuf::from(local)
                .join("RoomForge")
                .join("roomforge.log");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home)
            .join(".roomforge")
            .join("roomforge.log");
    }
    std::path::PathBuf::from("roomforge.log")
}

fn diag_log(line: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{line}");
    }
}

fn main() -> eframe::Result<()> {
    // No console in release builds: panics go to the local log so a
    // silent death is always explainable after the fact.
    std::panic::set_hook(Box::new(|info| {
        diag_log(&format!("PANIC: {info}"));
    }));
    diag_log(&format!("launch: RoomForge v{}", env!("CARGO_PKG_VERSION")));

    // Optional: `roomforge plan.roomforge` opens that plan.
    let arg_path = std::env::args().nth(1).map(std::path::PathBuf::from);

    let options = eframe::NativeOptions {
        // Glow (OpenGL), pinned explicitly — see DECISIONS.md.
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("RoomForge")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eframe::run_native(
            "RoomForge",
            options,
            Box::new(move |_cc| {
                let mut app = app::App::new();
                if let Some(path) = arg_path.clone() {
                    app.open_path(path);
                }
                Ok(Box::new(app) as Box<dyn eframe::App>)
            }),
        )
    }));
    match outcome {
        Ok(result) => result,
        Err(_) => {
            diag_log("fatal: panic during startup; see the PANIC line above");
            std::process::exit(1);
        }
    }
}
