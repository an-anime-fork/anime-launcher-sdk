use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

use crate::config::schema_blanks::prelude::*;
//use crate::traits::enhancementsbase::EnhancementsBase;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enhancements {
    //pub fsr: Fsr,
    pub gamemode: bool,
    pub hud: HUD,
    pub gamescope: Gamescope,
    pub steamrt: bool, // Steam Runtime active. Default to true.
    pub dx11: bool, // choice?
    pub xalia: bool, // Xalia glyphs
    pub obs_vkcapture: bool, // OBS Vulkan capture layer
    pub force_raytrace: bool, // GPU ID override for Ray Tracing
    pub fix_launch_dialog: bool
}

//impl EnhancementsBase for Enhancements {}

impl From<&JsonValue> for Enhancements {
    fn from(value: &JsonValue) -> Self {
        let default = Self::default();

        Self {
            /*
            fsr: value.get("fsr")
                .map(Fsr::from)
                .unwrap_or(default.fsr),
            */

            gamemode: value.get("gamemode")
                .and_then(JsonValue::as_bool)
                .unwrap_or(default.gamemode),

            hud: value.get("hud")
                .map(HUD::from)
                .unwrap_or(default.hud),

            gamescope: value.get("gamescope")
                .map(Gamescope::from)
                .unwrap_or(default.gamescope),

            steamrt: value.get("steamrt")
                .and_then(JsonValue::as_bool)
                .unwrap_or(true),

            xalia: value.get("xalia")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false),

            obs_vkcapture: value.get("obs_vkcapture")
                .and_then(JsonValue::as_bool)
                .unwrap_or(true),

            force_raytrace: value.get("force_raytrace")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false),

            dx11: value.get("dx11")
                .and_then(JsonValue::as_bool)
                .unwrap_or(true),

            fix_launch_dialog: value.get("fix_launch_dialog")
                .and_then(JsonValue::as_bool)
                .unwrap_or(true),
        }
    }
}
