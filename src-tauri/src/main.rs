#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod behavior;
mod data;
mod image_analyzer;
mod ipc;
mod movement;
mod platform;
mod utils;

use std::{
    fs::{self},
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use guard::guard;
use ipc::FrontendInfo;
use parking_lot::RwLock;
use slog::{Drain, Level, Logger};
use tauri::{Manager, Size, LogicalSize};

use crate::{
    behavior::{Behavior, FarmingBehavior, ShoutBehavior, SupportBehavior},
    image_analyzer::ImageAnalyzer,
    ipc::{BotConfig, BotMode},
    movement::MovementAccessor,
    utils::Timer,
};

struct AppState {
    logger: Logger,
}

fn main() {
    // Generate tauri context
    let context = tauri::generate_context!();
    let neuz_version = context
        .config()
        .package
        .version
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Setup logging
    let sentry_options = sentry::ClientOptions {
        release: Some(std::borrow::Cow::from(format!("v{}", neuz_version))),
        server_name: Some(std::borrow::Cow::from(format!("neuz v{}", neuz_version))),
        ..Default::default()
    };
    let _sentry = sentry::init((
        "https://ce726b0d4b634de8a44ab5564bc924fe@o1322474.ingest.sentry.io/6579555",
        sentry_options,
    ));
    let drain = {
        let decorator = slog_term::TermDecorator::new().stdout().build();
        let drain = slog_term::CompactFormat::new(decorator)
            .build()
            .filter_level(Level::Trace)
            .fuse();
        slog_async::Async::new(drain).build().fuse()
    };
    let drain = sentry_slog::SentryDrain::new(drain).fuse();
    let logger = Logger::root(drain.fuse(), slog::o!());

    // Build app
    tauri::Builder::default()
        // .menu(tauri::Menu::os_default(&context.package_info().name))
        .manage(AppState { logger })
        .invoke_handler(tauri::generate_handler![
            start_bot,
            create_window,
            get_profiles,
            create_profile,
            remove_profile,
            rename_profile,
            copy_profile,
            reset_profile
        ])
        .run(context)
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_profiles(_state: tauri::State<AppState>, app_handle: tauri::AppHandle) -> Vec<String> {
    drop(fs::create_dir(
        format!(
            r"{}\",
            app_handle
                .path_resolver()
                .app_dir()
                .unwrap()
                .to_string_lossy()
        ).clone(),
    ));
    let paths = fs::read_dir(format!(
        r"{}\",
        app_handle
            .path_resolver()
            .app_dir()
            .unwrap()
            .to_string_lossy()
    ))
    .unwrap();
    let mut profiles = vec![];

        for path in paths {
            if let Ok(entry) = path {
                if entry.file_name().to_str().unwrap().starts_with("profile_") {
                    profiles.push(String::from(&*entry.file_name().to_str().unwrap()));
                }
            }
        }
    if profiles.len() == 0 {
        drop(fs::create_dir(
            format!(
                r"{}\profile_DEFAULT",
                app_handle
                    .path_resolver()
                    .app_dir()
                    .unwrap()
                    .to_string_lossy()
            ).clone(),
        ));
        profiles.push("profile_DEFAULT".to_string());
    }


    profiles
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
fn config_folder_path(app_handle: &tauri::AppHandle, profile_id: &String) -> String {
    format!(
        r"{}\profile_{}",
        app_handle
            .path_resolver()
            .app_dir()
            .unwrap()
            .to_string_lossy(),
        profile_id
    )
}
fn config_file_path(app_handle: &tauri::AppHandle, profile_id: &String) -> String {
    format!(
        r"{}\.botconfig_{}",
        app_handle
            .path_resolver()
            .app_dir()
            .unwrap()
            .to_string_lossy(),
        profile_id
    )
}
#[tauri::command]
fn copy_profile(
    profile_id: String,
    new_profile_id: String,
    _state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) {
    drop(fs::copy(
        config_file_path(&app_handle, &profile_id),
        config_file_path(&app_handle, &new_profile_id).clone(),
    ));
    drop(copy_dir_all(
        config_folder_path(&app_handle, &profile_id),
        config_folder_path(&app_handle, &new_profile_id),
    ));
}

#[tauri::command]
fn create_profile(
    profile_id: String,
    _state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) {
    drop(fs::create_dir(config_folder_path(&app_handle, &profile_id)));
}

#[tauri::command]
fn remove_profile(
    profile_id: String,
    _state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) {
    drop(fs::remove_dir_all(config_folder_path(
        &app_handle,
        &profile_id,
    )));
    drop(fs::remove_file(config_file_path(&app_handle, &profile_id)));
}

#[tauri::command]
fn rename_profile(
    profile_id: String,
    new_profile_id: String,
    _state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) {
    drop(fs::rename(
        config_folder_path(&app_handle, &profile_id),
        config_folder_path(&app_handle, &new_profile_id),
    ));
    drop(fs::rename(
        config_folder_path(&app_handle, &profile_id),
        config_folder_path(&app_handle, &new_profile_id).clone(),
    ));
}

#[tauri::command]
fn reset_profile(profile_id: String, _state: tauri::State<AppState>, app_handle: tauri::AppHandle) {
    drop(fs::remove_dir_all(config_folder_path(
        &app_handle,
        &profile_id,
    )));
    drop(fs::remove_file(
        config_file_path(&app_handle, &profile_id).clone(),
    ));
    drop(fs::create_dir(
        config_folder_path(&app_handle, &profile_id).clone(),
    ));
}

#[tauri::command]
async fn create_window(profile_id: String, app_handle: tauri::AppHandle) {
    let window = tauri::WindowBuilder::new(
        &app_handle,
        "client",
        tauri::WindowUrl::External("https://universe.flyff.com/play".parse().unwrap()),
    )
    .data_directory(PathBuf::from(format!(
        r"{}\profile_{}",
        app_handle
            .path_resolver()
            .app_dir()
            .unwrap()
            .to_string_lossy(),
        profile_id
    )))
    //.resizable(false)
    .center()
    .inner_size(800.0, 600.0)
    .title(format!("{} | Flyff Universe", profile_id))
    .build()
    .unwrap();
    drop(window.show());

    let main_window = app_handle.get_window("main").unwrap();
    drop(main_window.set_title(format!("{} Neuz | MadrigalStreetCartel", profile_id).as_str()));
    //window.once_global("tauri://close-requested", move |_| app_handle.restart());
}

#[tauri::command]
fn start_bot(profile_id: String, state: tauri::State<AppState>, app_handle: tauri::AppHandle) {
    let logger = state.logger.clone();
    let config_path = format!(
        r"{}\.botconfig_{}",
        app_handle
            .path_resolver()
            .app_dir()
            .unwrap()
            .to_string_lossy(),
        profile_id
    )
    .clone();

    std::thread::spawn(move || {
        let logger = logger.clone();

        let mut last_config_change_id = 0;
        let config: Arc<RwLock<BotConfig>> =
            Arc::new(RwLock::new(BotConfig::deserialize_or_default(config_path)));

        // Listen for config changes from the UI
        let local_config = config.clone();
        let logger_botconfig_c2s = logger.clone();

        app_handle.listen_global("bot_config_c2s", move |e| {
            slog::trace!(logger_botconfig_c2s, "Received config change"; "event_payload" => e.payload());
            if let Some(payload) = e.payload() {
                match serde_json::from_str::<BotConfig>(payload) {
                    Ok(new_config) => {
                        *local_config.write() = new_config.changed();
                    }
                    Err(e) => {
                        slog::error!(logger_botconfig_c2s, "Failed to parse config change"; "error" => e.to_string(), "error_payload" => payload);
                    }
                }
            }
        });

        // Listen for bot activation state
        let local_config = config.clone();
        app_handle.listen_global("toggle_bot", move |_| {
            local_config.write().toggle_active();
        });

        let send_config = |config: &BotConfig| {
            drop(app_handle.emit_all("bot_config_s2c", &*config) as Result<(), _>)
        };

        let send_info = |config: &FrontendInfo| {
            drop(app_handle.emit_all("bot_info_s2c", &*config) as Result<(), _>)
        };

        // Wait a second for frontend to become ready
        std::thread::sleep(Duration::from_secs(1));

        // Send initial config to frontend
        send_config(&*config.read());

        let window = app_handle.get_window("client").unwrap();
        let mut image_analyzer: ImageAnalyzer = ImageAnalyzer::new(&window);
        image_analyzer.window_id = platform::get_window_id(&window).unwrap_or(0);

        // Create movement accessor
        let movement = MovementAccessor::new(window.clone() /*&accessor*/);

        // Instantiate behaviors
        let mut farming_behavior = FarmingBehavior::new(&logger, &movement, &window);
        let mut shout_behavior = ShoutBehavior::new(&logger, &movement, &window);
        let mut support_behavior = SupportBehavior::new(&logger, &movement, &window);

        let mut last_mode: Option<BotMode> = None;
        let mut frontend_info: Arc<RwLock<FrontendInfo>> =
            Arc::new(RwLock::new(FrontendInfo::deserialize_or_default()));
        send_info(&*frontend_info.read());
        // Enter main loop
        loop {
            let timer = Timer::start_new("main_loop");
            let config = &*config.read();
            let mut frontend_info_mut = *frontend_info.read();

            // Send changed config to frontend if needed
            if config.change_id() > last_config_change_id {
                config.serialize(
                    format!(
                        r"{}\.botconfig_{}",
                        app_handle
                            .path_resolver()
                            .app_dir()
                            .unwrap()
                            .to_string_lossy(),
                        profile_id
                    )
                    .clone(),
                );
                send_config(config);
                last_config_change_id = config.change_id();

                // Update behaviors
                farming_behavior.update(config);
                shout_behavior.update(config);
                support_behavior.update(config);
            }

            // Client window is closed
            if window.is_resizable().is_err() {
                app_handle.restart();
                break;
            }

            // Continue early if the bot is not engaged
            if !config.is_running() {
                if !window.is_resizable().unwrap() {
                    drop(window.set_resizable(true));
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
                timer.silence();
                continue;
            }

            if !config.farming_config().is_stop_fighting() {
               drop(window.set_size(Size::Logical(LogicalSize {width: 800.0, height: 600.0})));
               drop(window.set_resizable(false));
             }else {
                drop(window.set_resizable(true));
             }

            frontend_info_mut.set_is_running(true);

            // Make sure an operation mode is set
            guard!(let Some(mode) = config.mode() else {
                std::thread::sleep(std::time::Duration::from_millis(100));
                timer.silence();
                continue;
            });

            // Check if mode is different from last mode
            if let Some(last_mode) = last_mode.as_ref() {
                if &mode != last_mode {
                    slog::info!(logger, "Mode changed"; "old_mode" => last_mode.to_string(), "new_mode" => mode.to_string());

                    // Stop all behaviors
                    farming_behavior.stop(&config);
                    support_behavior.stop(&config);
                    shout_behavior.stop(&config);

                    // Start the current behavior
                    match mode {
                        BotMode::Farming => farming_behavior.start(&config),
                        BotMode::Support => support_behavior.start(&config),
                        BotMode::AutoShout => shout_behavior.start(&config),
                    }
                }
            }

            // Capture client window
            image_analyzer.capture_window(&logger, config.farming_config());

            // Try capturing the window contents
            if image_analyzer.image_is_some() {
                // Update stats
                image_analyzer
                    .client_stats
                    .update(&image_analyzer.clone(), &logger);

                // Run the current behavior
                guard!(let Some(mode) = config.mode() else { continue; });

                // Stop bot in case of death
                let is_alive = image_analyzer.client_stats.is_alive();
                if !is_alive {
                    frontend_info_mut.set_is_alive(false);
                    //frontend_info = Arc::new(RwLock::new(frontend_info_mut));
                    continue;
                } else if is_alive && !frontend_info_mut.is_alive() {
                    frontend_info_mut.set_is_alive(true);
                }
                match mode {
                    BotMode::Farming => {
                        farming_behavior.run_iteration(
                            &mut frontend_info_mut,
                            config,
                            &mut image_analyzer,
                        );
                    }
                    BotMode::AutoShout => {
                        shout_behavior.run_iteration(
                            &mut frontend_info_mut,
                            config,
                            &mut image_analyzer,
                        );
                    }
                    BotMode::Support => {
                        support_behavior.run_iteration(
                            &mut frontend_info_mut,
                            config,
                            &mut image_analyzer,
                        );
                    }
                }
                frontend_info = Arc::new(RwLock::new(frontend_info_mut));
                // Send infos to frontend
                send_info(&*frontend_info.read());
            }

            // Update last mode
            last_mode = config.mode();
        }
    });
}
