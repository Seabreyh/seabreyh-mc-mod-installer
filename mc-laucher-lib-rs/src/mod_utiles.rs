use crate::expections::{LauncherLibError, LibResult};
use crate::utils::{download_file_to_string, get_http_client};
use serde::Deserialize;

use crate::json::install::{Callback, Event};
use crate::utils::download_file;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

fn get_jar_name(url: &str) -> &str {
    let dir_tree: Vec<&str> = url.split(&['\\', '/'][..]).collect();
    *dir_tree.last().unwrap()
}

fn get_mod_name_id(jar: &str) -> Option<String> {
    let mut mod_name = None;
    let words = jar.split(&['-', '_', '+', '.', '='][..]);
    for (i, word) in words.clone().enumerate() {
        if word.to_lowercase() == "forge" || word.matches(char::is_numeric).last().is_some() {
            let name_parts = &words.clone().collect::<Vec<&str>>()[0..i];
            mod_name = Some(name_parts.concat());
            break;
        }
    }

    mod_name
}

pub async fn install_mods(mc_dir: PathBuf, callback: Callback) -> LibResult<()> {
    let mut mods_list = String::new();
    if let Err(err) = download_file_to_string(
        "https://raw.githubusercontent.com/Seabreyh/seabreyh-mc-mod-installer/main/assets/mods.md"
            .to_string(),
        &mut mods_list,
        callback,
    )
    .await
    {
        return Err(err);
    }

    let mods_dir = mc_dir.join("mods");

    let mods_backup = mc_dir.join("mods.backup");

    if let Err(_) = dircpy::copy_dir(mods_dir.clone(), mods_backup) {
        std::fs::create_dir(&mods_dir).expect("Failed to create .minecraft/mods folder");
    }

    let mut to_install_mod_jars = mods_list.lines().fold(HashMap::new(), |mut acc, file_url| {
        *acc.entry(get_jar_name(file_url)).or_insert(file_url) = file_url;
        acc
    });

    let to_install_mod_ids = to_install_mod_jars
        .iter()
        .filter_map(|(&jar, _)| get_mod_name_id(jar))
        .collect::<HashSet<String>>();

    // Iterate over the already installed jars to see if we can avoid installing it again
    let installed_jars = fs::read_dir(&mods_dir).unwrap();
    for jar_path in installed_jars.filter_map(|s| {
        let file_path = s.as_ref().unwrap().path().display().to_string();
        file_path.contains(".jar").then_some(file_path)
    }) {
        let jar = get_jar_name(&jar_path);

        if to_install_mod_jars.contains_key(jar) {
            to_install_mod_jars.remove(jar);
            callback(Event::Status(format!("Skipping mod {}", jar)));
        } else if let Some(mod_id) = get_mod_name_id(jar) {
            if to_install_mod_ids.contains(&mod_id) {
                let mod_file = mods_dir.join(jar);
                callback(Event::Status(format!("Replacing out-dated mod {}", jar)));
                fs::remove_file(mod_file)
                    .expect(&format!("Error: couldn't remove out-dated mod {}", jar));
            }
        }
    }

    // Install mods from mods.txt
    for (jar, download_url) in to_install_mod_jars {
        callback(Event::Status(format!(
            "mod_name={:?}",
            get_mod_name_id(jar)
        )));

        let mod_file = mods_dir.join(jar);
        if let Err(err) = download_file(
            download_url.to_string(),
            mod_file.clone(),
            callback,
            None,
            false,
        )
        .await
        {
            return Err(err);
        }

        callback(Event::Status(format!(
            "Installed mod {}",
            mod_file.display()
        )));
    }

    // Install shaderpack
    let shaderpacks_dir = mc_dir.join("shaderpacks");

    let shaderpacks_backup = mc_dir.join("shaderpacks.backup");

    if let Err(_) = dircpy::copy_dir(&shaderpacks_dir, shaderpacks_backup) {
        std::fs::create_dir(&shaderpacks_dir)
            .expect("Failed to create .minecraft/shaderpacks folder");
    }
    let shader_url =
        "https://mediafiles.forgecdn.net/files/3928/682/ComplementaryReimagined_r1.2.2.zip";
    let url_tree: Vec<&str> = shader_url.split(&['/', '='][..]).collect();
    let shaderpack = *url_tree.last().unwrap();
    let shaderpack_file = shaderpacks_dir.join(shaderpack);
    if let Err(err) = download_file(
        shader_url.to_string(),
        shaderpack_file.clone(),
        callback,
        None,
        false,
    )
    .await
    {
        return Err(err);
    }

    callback(Event::Status(format!(
        "Installed shader {}",
        shaderpack_file.display()
    )));

    // Install shader settings
    let shader_settings_txt = include_str!("../../assets/ComplementaryReimagined_r1.2.2.zip.txt");

    let shaderpack_settings_txt_path =
        shaderpacks_dir.join("ComplementaryReimagined_r1.2.2.zip.txt");
    let mut shaderpack_settings_txt_file = File::create(shaderpack_settings_txt_path).unwrap();
    // writeln!(&mut shaderpack_settings_txt_file, "{}", shader_settings_txt).unwrap();
    shaderpack_settings_txt_file
        .write_fmt(format_args!("{}", shader_settings_txt))
        .expect("Failed to write shaderpack settings .txt");

    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
pub struct Version {
    #[serde(rename = "$value")]
    pub version: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MavenVersioning {
    pub latest: String,
    pub release: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    pub versions: Version,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MavenMetadata {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    pub versioning: MavenVersioning,
}

pub async fn get_metadata(root_url: &str) -> LibResult<MavenMetadata> {
    let client = match get_http_client().await {
        Ok(value) => value,
        Err(err) => return Err(err),
    };

    match client
        .get(format!("{}maven-metadata.xml", root_url).as_str())
        .send()
        .await
    {
        Ok(response) => match response.text().await {
            Ok(value) => match serde_xml_rs::from_str::<MavenMetadata>(&value) {
                Ok(res) => Ok(res),
                Err(err) => Err(LauncherLibError::General(err.to_string())),
            },
            Err(err) => Err(LauncherLibError::PraseJsonReqwest(err)),
        },
        Err(err) => Err(LauncherLibError::HTTP {
            source: err,
            msg: "Failed to make http request".into(),
        }),
    }
}
