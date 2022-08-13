use crate::expections::{LauncherLibError, LibResult};
use crate::utils::get_http_client;
use serde::Deserialize;

use crate::json::install::{Callback, Event};
use crate::utils::download_file;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::path::PathBuf;

fn read_mods_file<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub async fn install_mods(mc_dir: PathBuf, callback: Callback) -> LibResult<()> {
    let mods_dir = mc_dir.join("mods");

    let mods_backup = mc_dir.join("mods.backup");
    dircpy::copy_dir(mods_dir.clone(), mods_backup).unwrap();

    if let Ok(lines) = read_mods_file(".\\mods.txt") {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(mod_url) = line {
                let url_tree: Vec<&str> = mod_url.split("/").collect();
                let jar = *url_tree.last().unwrap();
                let mod_file = mods_dir.join(jar);
                if let Err(err) =
                    download_file(mod_url.clone(), mod_file.clone(), callback, None, false).await
                {
                    return Err(err);
                }

                callback(Event::Status(format!(
                    "Installed mod {}",
                    mod_file.display()
                )));
            }
        }
    }

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
