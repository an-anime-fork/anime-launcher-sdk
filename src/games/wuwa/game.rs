use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::path::PathBuf;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::env;

use anime_game_core::wuwa::telemetry;

use crate::components::wine::Bundle as WineBundle;

use crate::config::ConfigExt;
use crate::wuwa::config::Config;
use crate::games::common;
use crate::wuwa::config::schema::game::Game;

use crate::config::schema_blanks::prelude::{
    WineDrives,
    AllowedDrives
};
use crate::integrations::steam::steam_managed_game_install_executable;
use crate::traits::integrations::steamgame::SteamGame;
use crate::wuwa::consts;
use crate::wuwa::states::LauncherStateParams;

#[derive(Debug, Clone)]
struct Folders {
    pub wine: PathBuf,
    pub prefix: PathBuf,
    pub game: PathBuf,
    pub patch: PathBuf,
    pub temp: PathBuf
}

fn replace_keywords(command: impl ToString, folders: &Folders) -> String {
    command.to_string()
        .replace("%build%", folders.wine.to_str().unwrap())
        .replace("%prefix%", folders.prefix.to_str().unwrap())
        .replace("%temp%", folders.game.to_str().unwrap())
        .replace("%launcher%", &consts::launcher_dir().unwrap().to_string_lossy())
        .replace("%game%", folders.temp.to_str().unwrap())
}

#[cfg(feature = "steam")]
impl SteamGame for Game {
    const STEAM_GAME_ID: i32 = 3513350;
    fn has_steam_game_entry(&self) -> bool { true }
}

/// Try to run the game
///
/// This function will freeze thread it was called from while the game is running
#[tracing::instrument(level = "info", ret)]
pub fn run() -> anyhow::Result<()> {
    tracing::info!("Preparing to run the game");

    let config = Config::get()?;
    let game_exec = match steam_managed_game_install_executable() {
        None => config.game.path.for_edition(config.launcher.edition).to_path_buf(),
        Some(path) => path
    };
    let game_path = match String::from(game_exec.to_string_lossy()).ends_with(".exe") {
        true => game_exec.parent().unwrap(),
        false => game_exec.as_path()
    };

    if !game_path.exists() {
        return Err(anyhow::anyhow!("Game is not installed"));
    }

    let Some(wine) = config.get_selected_wine()? else {
        anyhow::bail!("Couldn't find wine executable");
    };

    let features = wine.features(&config.components.path)?.unwrap_or_default();

    let mut folders = Folders {
        wine: wine.get_runner_dir(config.game.wine.builds.clone()),
        prefix: wine.get_prefix_dir(config.game.wine.prefix.clone()),
        game: PathBuf::from(game_path),
        patch: config.patch.path.clone(),
        temp: config.launcher.temp.clone().unwrap_or(std::env::temp_dir())
    };

    // Prepare wine prefix drives
    let prefix_folder = config.get_wine_prefix_path();

    config.game.wine.drives.map_folders(&folders.game, &prefix_folder)?;

    // Workaround for the jadeite patch (we run it from Z: drive)
    WineDrives::map_folder(&prefix_folder, AllowedDrives::Z, "/")?;

    // Workaround for sandboxing feature
    if config.sandbox.enabled {
        WineDrives::map_folder(&prefix_folder, AllowedDrives::C, "../drive_c")?;
    }

    // Prepare bash -c '<command>'
    // %command% = %bash_command% %windows_command% %launch_args%

    let mut bash_command_vec: Vec<String> = vec![];
    let mut launch_args_vec: Vec<String> = vec![];

    if config.game.enhancements.gamemode {
        bash_command_vec.push("gamemoderun".to_string());
    }

    let run_command = features.command
        .map(|command| replace_keywords(command, &folders))
        .unwrap_or(format!("\"{}\"", folders.wine.join(wine.files.wine64.unwrap_or(wine.files.wine)).to_string_lossy()));

    bash_command_vec.push(run_command);

    // gamescope <params> -- <command to run>
    if let Some(gamescope) = config.game.enhancements.gamescope.get_command() {
        bash_command_vec.insert(0, "--".to_string());
        bash_command_vec.insert(0, gamescope.to_string());
    }

    // nahhhhhhhhhhh
    launch_args_vec.push((if config.game.enhancements.dx11 {"-dx11"} else {"-dx12"} ).into());

    let windows_command = format!("\"{}\"", game_exec.to_string_lossy());
    let bash_command = match &config.game.command {
        // Use user-given launch command
        Some(command) => replace_keywords(command, &folders)
            .replace("%command%", &format!("{} {windows_command} {}", bash_command_vec.join(" "), launch_args_vec.join(" ")))
            .replace("%bash_command%", bash_command_vec.join(" ").as_str())
            .replace("%windows_command%", &windows_command)
            .replace("%launch_args%", launch_args_vec.join(" ").as_str()),

        // Combine bash and windows parts of the command
        None => format!("{} {windows_command} {}", bash_command_vec.join(" "), launch_args_vec.join(" "))
    };

    let mut command = Command::new("bash");

    command.arg("-c");
    command.arg(&bash_command);

    // Game ID per Steam. Just set it in.
    if config.game.was_launched_from_steam_game() && config.game.has_steam_game_entry() {
        for envvar in [
            "STEAM_COMPAT_APP_ID", "SteamAppId", "SteamGameId", "SteamOverlayGameId"
        ].iter() {
            command.env(envvar, Game::STEAM_GAME_ID.to_string());
        }
        // Env that just gets set to 1
        for envvar in [
            "STEAM_COMPAT_PROTON",                              // force indicate we're Proton
            config.game.get_deck_or_steamos_env_var().as_str()  // Ask the game nicely
        ].iter() {
            command.env(envvar, "1");
        }
    }

    // Setup environment
    command.env("WINEARCH", "win64");
    command.env("WINEDLLOVERRIDES", "KRSDKExternal.exe=d");

    //common::generic_wine_checks()

    // Vulkan accelerated capture layer
    if config.game.enhancements.obs_vkcapture {
        command.env("OBS_VKCAPTURE", "1");
    }
    // Xalia Glyph library
    if ! config.game.enhancements.xalia {
        command.env("PROTON_USE_XALIA", "0");
    }
    // Raytrace override
    if config.game.enhancements.force_raytrace {
        command.env(
            "DXVK_CONFIG",
            "dxgi.customDeviceDesc=\"NVIDIA GeForce RTX 4090\";dxgi.customDeviceId=2684;dxgi.customVendorId=10de"
        );
    }

    // Add environment flags for selected wine
    for (key, value) in features.env.into_iter() {
        command.env(key, replace_keywords(value, &folders));
    }

    // Add environment flags for selected dxvk, if used downstream
    if let Ok(Some(dxvk)) = config.get_selected_dxvk() {
        if let Ok(Some(features)) = dxvk.features(&config.components.path) {
            for (key, value) in features.env.iter() {
                command.env(key, replace_keywords(value, &folders));
            }
        }
    }

    let mut wine_folder = folders.wine.clone();

    if features.bundle == Some(WineBundle::Proton) {
        wine_folder.push("files");
    }

    command.envs(config.game.enhancements.hud.get_env_vars(config.game.enhancements.gamescope.enabled));
    //command.envs(config.game.enhancements.fsr.get_env_vars());

    command.envs(config.game.wine.sync.get_env_vars());
    command.envs(config.game.wine.language.get_env_vars());
    command.envs(config.game.wine.shared_libraries.get_env_vars(wine_folder));

    command.envs(&config.game.environment);

    // Run command

    let variables = command
        .get_envs()
        .map(|(key, value)| {
            format!("{}=\"{}\"", key.to_string_lossy(), value.unwrap_or_default().to_string_lossy())
        })
        .fold(String::new(), |acc, env| acc + " " + &env);

    tracing::info!("Running the game with command: {variables} {bash_command}");

    // We use real current dir here because sandboxed one
    // obviously doesn't exist
    let mut child = command.current_dir(game_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Create new game.log file to log all the game output
    let game_output = Arc::new(Mutex::new(
        File::create(consts::launcher_dir()?.join("game.log"))?
    ));

    let written = Arc::new(AtomicUsize::new(0));

    let mut stdout_join = None;
    let mut stderr_join = None;

    // Redirect stdout to the game.log file
    if let Some(mut stdout) = child.stdout.take() {
        let game_output = game_output.clone();
        let written = written.clone();

        stdout_join = Some(std::thread::spawn(move || -> std::io::Result<()> {
            let mut buf = [0; 1024];

            while let Ok(read) = stdout.read(&mut buf) {
                if read == 0 {
                    break;
                }

                let Ok(mut game_output) = game_output.lock() else {
                    break;
                };

                for line in buf[..read].split(|c| c == &b'\n') {
                    game_output.write_all(b"    [stdout] ")?;
                    game_output.write_all(line)?;
                    game_output.write_all(b"\n")?;

                    written.fetch_add(line.len() + 14, Ordering::Relaxed);
                }

                if written.load(Ordering::Relaxed) > *consts::GAME_LOG_FILE_LIMIT {
                    break;
                }
            }

            Ok(())
        }));
    }

    // Redirect stderr to the game.log file
    if let Some(mut stderr) = child.stderr.take() {
        let game_output = game_output.clone();
        let written = written.clone();

        stderr_join = Some(std::thread::spawn(move || -> std::io::Result<()> {
            let mut buf = [0; 1024];

            while let Ok(read) = stderr.read(&mut buf) {
                if read == 0 {
                    break;
                }

                let Ok(mut game_output) = game_output.lock() else {
                    break;
                };

                for line in buf[..read].split(|c| c == &b'\n') {
                    game_output.write_all(b"[!] [stderr] ")?;
                    game_output.write_all(line)?;
                    game_output.write_all(b"\n")?;

                    written.fetch_add(line.len() + 14, Ordering::Relaxed);
                }

                if written.load(Ordering::Relaxed) > *consts::GAME_LOG_FILE_LIMIT {
                    break;
                }
            }

            Ok(())
        }));
    }

    child.wait()?;


    // Flush and close the game log file
    if let Ok(mut file) = game_output.lock() {
        file.flush()?;
    }

    drop(game_output);

    if let Some(join) = stdout_join {
        join.join().map_err(|err| {
            anyhow::anyhow!("Failed to join stdout reader thread: {err:?}") }
        )??;
    }

    if let Some(join) = stderr_join {
        join.join().map_err(|err| {
            anyhow::anyhow!("Failed to join stderr reader thread: {err:?}") }
        )??;
    }

    // Workaround for fast process closing (is it still a thing?)
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3));

        let output = Command::new("ps").arg("-A").stdout(Stdio::piped()).output()?;
        let output = String::from_utf8_lossy(&output.stdout);

        if !output.contains("Client-Win64-Sh") {
            break;
        }
    }

    let ret_status = child.wait()?;

    // Encountered codes:
    //  - 0 (standard exit)
    //  - ? (patched?)
    //  - ? (anticheat crashout)
    tracing::info!("{}", &format!("Known exit code: {}", ret_status.code().unwrap_or(-1)));

    Ok(())
}
