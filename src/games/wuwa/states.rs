use std::path::PathBuf;

use anime_game_core::prelude::*;
use anime_game_core::wuwa::prelude::*;

use crate::config::ConfigExt;
use crate::integrations::steam;
use crate::integrations::steam::LaunchedFrom;

/**
 * TODO: Review this whole spec and do away with version checks and update checks.
 * This class needs to get Jadeite ripped out of it to be used as an intermediate
 * launcher for Wuthering Waves on Steam. Upstream install data is to be trusted
 * automatically
 **/

#[derive(Debug, Clone)]
pub enum LauncherState {
    Launch,

    // Todo: yeet this and replace with Proton select
    #[cfg(feature = "components")]
    WineNotInstalled,

    PrefixNotExists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateUpdating {
    Components,
    Game,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherStateParams<F: Fn(StateUpdating)> {
    pub game_path: PathBuf,
    pub game_edition: GameEdition,

    pub wine_prefix: PathBuf,

    pub fast_verify: bool,
    pub status_updater: F
}

impl LauncherState {
    pub fn get<F: Fn(StateUpdating)>(params: LauncherStateParams<F>) -> anyhow::Result<Self> {
        tracing::debug!("Trying to get launcher state");

        // Check wine components installation status
        (params.status_updater)(StateUpdating::Components);

        // Check game installation status
        (params.status_updater)(StateUpdating::Game);

        let game = Game::new(&params.game_path, params.game_edition)
            .with_fast_verify(params.fast_verify);

        // TODO: wine selection check?
        return Ok(Self::Launch);
    }

    #[cfg(feature = "config")]
    pub fn get_from_config<T: Fn(StateUpdating)>(status_updater: T) -> anyhow::Result<Self> {
        tracing::debug!("Trying to get launcher state");

        let config = crate::wuwa::config::Config::get()?;

        //        match &config.game.wine.selected {
        match config.get_selected_wine()? {
            #[cfg(feature = "components")]
            Some(selected) => {
                if selected.managed {
                    tracing::debug!(
                        "DEBUG: runner dir {:?} :: managed somehow",
                        selected.get_runner_dir(config.game.wine.builds.clone())
                    );
                }
            },
            None => {
                // noop
            }
        }

        match steam::launched_from() {
            LaunchedFrom::Steam => {
                match &config.game.wine.selected {
                    #[cfg(feature = "components")]
                    Some(selected) if !steam::valid_selected_runner(selected)
                        => return Ok(Self::WineNotInstalled),
                    None
                        => return Ok(Self::WineNotInstalled),
                    _
                    => ()
                }
            },
            LaunchedFrom::Independent => {
                match &config.game.wine.selected {
                    #[cfg(feature = "components")]
                    Some(selected) if !config.game.wine.builds.join(selected).exists()
                        => return Ok(Self::WineNotInstalled),
                    None
                        => return Ok(Self::WineNotInstalled),
                    _
                    => ()
                }
            }
        }
        let mut init_path = config.game.path.for_edition(config.launcher.edition).to_path_buf();
        match steam::steam_managed_game_install_executable() {
            None => (),
            Some(executable ) => {
                init_path = executable;
            }
        }

        Self::get(LauncherStateParams {
            game_path: init_path,
            game_edition: config.launcher.edition,

            wine_prefix: config.get_wine_prefix_path(),

            fast_verify: config.launcher.repairer.fast,

            status_updater
        })
    }
}
