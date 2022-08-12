use std::fs::File;

use run_script::ScriptOptions;
use serde::{Deserialize, Serialize};

const FERIUM_CONFIG_FILE: &str = "ferium_config.json";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ModIdentifier {
    curse_forge_project: i32,
}
#[derive(Serialize, Deserialize)]
struct Mod {
    name: String,
    identifier: ModIdentifier,
}

#[derive(Serialize, Deserialize)]
struct Profile {
    name: String,
    output_dir: String,
    game_version: String,
    mod_loader: String,
    mods: Vec<Mod>,
}

#[derive(Serialize, Deserialize)]
struct FeriumConfigJSON {
    active_profile: i32,
    active_modpack: i32,
    profiles: Vec<Profile>,
    modpacks: Vec<String>,
}

fn get_mods() -> Vec<Mod> {
    vec![
        Mod {
            name: "Create".to_string(),
            identifier: ModIdentifier {
                curse_forge_project: 328085,
            },
        },
        Mod {
            name: "Flywheel".to_string(),
            identifier: ModIdentifier {
                curse_forge_project: 486392,
            },
        },
    ]
}

pub fn create_config(outdir: &str, profile_name: &str, game_version: &str, mod_loader: &str) {
    // Some data structure.
    let config = FeriumConfigJSON {
        active_profile: 0,
        active_modpack: 0,
        profiles: vec![Profile {
            name: profile_name.to_string(),
            output_dir: outdir.to_string(),
            game_version: game_version.to_string(),
            mod_loader: mod_loader.to_string(),
            mods: get_mods(),
        }],
        modpacks: vec![],
    };

    serde_json::to_writer(&File::create(FERIUM_CONFIG_FILE).unwrap(), &config).unwrap();
}

pub fn install_mods() {
    let options = ScriptOptions::new();

    let args = vec![];

    // run the script and get the script execution output
    let (exit_code, _, _) = run_script::run(
        &format!("ferium --config-file {} upgrade", FERIUM_CONFIG_FILE),
        &args,
        &options,
    )
    .expect("Error: failed to run ferium");
    if exit_code != 0 {
        panic!("Error: failed to execute ferium upgrade");
    }
}
