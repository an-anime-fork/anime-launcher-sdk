use crate::integrations::steam;

pub trait SteamGame {
    const STEAM_GAME_ID: i32 = -1;
    fn has_steam_game_entry() -> bool { false }

    fn was_launched_from_steam_game() -> bool { steam::launched_from_steam() }

}
