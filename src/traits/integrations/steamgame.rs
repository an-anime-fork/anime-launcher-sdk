use crate::integrations::steam;

pub trait SteamGame {
    const STEAM_GAME_ID: i32 = -1;
    fn has_steam_game_entry(&self) -> bool { false }

    fn was_launched_from_steam_game(&self) -> bool { steam::launched_from_steam() }
    fn get_deck_or_steamos_env_var(&self) -> String {
        match steam::is_steam_deck() {
            true => "SteamDeck".to_string(), // Hardware Check *literally* says we're Deck
            false => "SteamOS".to_string()   // Hardware Check says we're not a Deck
        }
    }

}
