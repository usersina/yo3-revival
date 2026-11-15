#![cfg_attr(windows, windows_subsystem = "windows")]
//! YO3 Revival, the player's launcher. The window is built from the skin kit (`yo3-skin`); everything it does with
//! the game, the server and the disk lives in this crate's library (`yo3_desktop`).
mod app;
mod error_presentation;
mod external;
mod home;
mod setup;

use std::sync::Mutex;
use yo3_desktop::{instance, updates};

fn main() -> iced::Result {
    let result = run();
    if let Err(error) = &result {
        startup_error(&format!("Could not open the application window.\n{error}\n\nKeep the complete Desktop bundle together and use a supported OS. Include this error and your version when asking for help."));
    }
    result
}

fn startup_error(message: &str) {
    eprintln!("{message}");
    rfd::MessageDialog::new().set_title("YO3 Revival could not open").set_description(message)
        .set_level(rfd::MessageLevel::Error).set_buttons(rfd::MessageButtons::Ok).show();
}

fn run() -> iced::Result {
    // The same executable runs elevated to set up direct start in a protected game folder.
    if let Some(code) = yo3_desktop::privileged::helper_mode() { std::process::exit(code); }
    let root = yo3_desktop::default_root();
    let startup = match updates::activation::Startup::parse(&root, std::env::args().skip(1)) {
        Ok(startup) => startup,
        Err(error) => { startup_error(&format!("Invalid startup request: {error:#}\nOpen the launcher normally from its complete Desktop folder.")); std::process::exit(1); }
    };
    // An older copy opened from a shortcut hands over to the installed update before anything else.
    let mut routing_error = None;
    if startup.readiness.is_none() && root.is_dir() {
        let routed = (|| -> anyhow::Result<bool> {
            let Some(target) = updates::activation::target(&root, yo3_tunnel::BUILD_VERSION, &std::env::current_exe()?)? else { return Ok(false) };
            updates::activation::launch(&root, &target, false, std::time::Duration::from_secs(25))?;
            Ok(true)
        })();
        match routed {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(error) => routing_error = Some(format!("Could not open the installed update. Using this version: {error:#}")),
        }
    }
    // One launcher per user. An update restart opens while the previous version closes, so it claims later.
    let claim = if startup.readiness.is_some() { app::Claim::AfterUpdate } else {
        match instance::claim(&root) {
            Ok(instance::Start::Owner(owner)) => app::Claim::Now(owner),
            Ok(instance::Start::Forwarded) => return Ok(()),
            Err(error) => { startup_error(&format!("{error:#}")); return Ok(()) }
        }
    };
    let boot = Mutex::new(Some((root, startup, routing_error, claim)));
    let mut application = iced::application(
        move || {
            let (root, startup, routing_error, claim) = boot.lock().expect("boot").take().expect("the application boots once");
            app::App::boot(root, startup, routing_error, claim)
        },
        app::App::update, app::App::view)
        .title("YO3 Revival").default_font(yo3_skin::BODY).subscription(app::App::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1024.0, 640.0), resizable: false, decorations: false, position: iced::window::Position::Centered,
            icon: iced::window::icon::from_rgba(include_bytes!(concat!(env!("OUT_DIR"), "/launcher-icon.rgba")).to_vec(), 256, 256).ok(),
            ..Default::default()
        })
        .exit_on_close_request(false);
    for font in yo3_skin::FONTS { application = application.font(font) }
    application.run()
}
