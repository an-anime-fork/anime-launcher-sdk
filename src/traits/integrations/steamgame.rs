
pub trait SteamGame {
    const STEAM_GAME_ID: i32 = -1;
    fn has_steam_game_entry(&self) -> bool { false }
}
