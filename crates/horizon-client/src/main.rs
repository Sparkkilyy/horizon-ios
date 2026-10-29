//! Horizon: native Rust client for Minecraft Java Edition 26.3 gameplay.
#![forbid(unsafe_code)]
mod game;

use horizon_client::{crash, logging};

/// The mod API level this game serves, kept in the program for the launcher
/// to read without running it (`horizon_mods::api_level::MARK`). `main`
/// reads it, so the linker keeps it.
static MOD_API_LEVEL: [u8; 27] = horizon_mods::api_level::MARK;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    horizon_client::telemetry::mark_start();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let game_args = args.iter().take_while(|a| *a != "--dedicated");
    let help = game_args.clone().any(|a| a == "--help" || a == "-h");
    // The dedicated server logs in its own folder, as vanilla's does; the game in its home.
    let (home, side, thread) = if args.iter().any(|a| a == "--dedicated") {
        (std::path::PathBuf::from("."), "server", "Server thread")
    } else {
        (logging::game_home(), "client", "Render thread")
    };
    #[cfg(target_os = "ios")]
    ios_home(&home);
    if !help {
        logging::init(&home, thread);
        // The log's first line names the build, so a bug report that carries the log says which one it was.
        log::info!("Horizon {}", crash::version());
        crash::install(home.clone(), side);
    }
    if let Err(error) = game::run() {
        log::error!("Horizon: {error}");
        if !help {
            crash::report_error(&home, side, &*error);
        }
        // Started from Explorer, the console window closes with the game:
        // keep the message on screen until it is read.
        #[cfg(windows)]
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            eprintln!("Press Enter to close.");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        std::process::exit(1);
    }
    logging::flush();
}

/// iOS starts the app in `/`, and no launcher sets the working directory the game reads its
/// options, saves and caches from: run in the game home (Documents) and say where the player's
/// own prepared resources go.
#[cfg(target_os = "ios")]
fn ios_home(home: &std::path::Path) {
    horizon_client::clipboard::set_ios_pasteboard(horizon_ios::copy_text, horizon_ios::pasted_text);
    let _ = std::fs::create_dir_all(home);
    if let Err(error) = std::env::set_current_dir(home) {
        eprintln!("Horizon: can't enter {}: {error}", home.display());
    }
    let note = home.join("PUT RESOURCES HERE.txt");
    if home.join("resources/horizon-resources.json").exists() {
        let _ = std::fs::remove_file(note);
    } else {
        let _ = std::fs::write(
            note,
            "Copy the prepared resources folder from your computer here and name it \"resources\".\n\
             On Windows it is %APPDATA%\\horizon-launcher\\versions\\<version>\\.local\\horizon-client\\26.3\n\
             (it holds horizon-resources.json). Then start Horizon again.\n",
        );
    }
}
