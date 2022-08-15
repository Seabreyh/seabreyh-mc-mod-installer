use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{fs::File, io::Cursor};

use iced::window::Icon;

use image::{EncodableLayout, GenericImageView};
use mc_laucher_lib_rs::{
    client::ClientBuilder,
    json::client::{InstallManifest, Loader},
};
use quartz_nbt::io::{read_nbt, write_nbt, Flavor};
use quartz_nbt::serde::{deserialize, serialize};
use quartz_nbt::NbtCompound;

use serde::{Deserialize, Serialize};

const TMP_MOD_DOWNLOAD_DIR: &str = "Downloads\\";
const GAME_VERSION: &str = "1.18.2";
const MC_SERVER_DAT_PATH: &str = ".minecraft\\servers.dat";
const MC_LAUNCHER_PROFILE_PATH: &str = ".minecraft\\launcher_profiles.json";
const LAUNCH_PROFILE_NAME: &str = "Seabreyh Mods";

const SERVER_NAME: &str = "Seabreyh MC Server";
const SERVER_IP: &str = "seabreyh.ml";

async fn run_install(user_path: PathBuf, roaming_path: PathBuf) {
    add_server_to_client(roaming_path.clone());
    install_forge_client_and_mods(user_path).await;
    set_launcher_profile(roaming_path);
    println!("Install complete!");
}

#[tokio::main]
pub async fn main() {
    let icon_bytes = include_bytes!("..\\icon.png");
    let image = image::load_from_memory(icon_bytes).expect("Could not load icon");
    let rgba = image.to_rgba8();
    let dimensions = image.dimensions();
    let _icon = Icon::from_rgba(rgba.as_bytes().to_vec(), dimensions.0, dimensions.1).ok();

    let user_dir = dirs::home_dir().unwrap();
    let roaming_dir = dirs::config_dir().unwrap();

    run_install(user_dir, roaming_dir).await;

    loop {}
}

async fn install_forge_client_and_mods(user_path: PathBuf) {
    if let Err(e) = ClientBuilder::install(
        InstallManifest::new(GAME_VERSION.into(), Loader::Forge),
        None,
        |event| {
            println!("{}", event);
        },
        Some(PathBuf::from(format!(
            "{}\\{}",
            user_path.display(),
            TMP_MOD_DOWNLOAD_DIR
        ))),
        None,
        None,
    )
    .await
    {
        panic!("Error during forge installation task: {}", e);
    }
}

fn add_server_to_client(roaming_dir: PathBuf) {
    fn get_file_as_byte_vec(filename: &str) -> Vec<u8> {
        let mut f_open = if let Ok(f) = File::open(filename) {
            f
        } else {
            File::create(filename).unwrap();
            return vec![];
        };
        let metadata = fs::metadata(filename).expect("unable to read metadata");
        let mut buffer = vec![0; metadata.len() as usize];
        f_open.read(&mut buffer).expect("buffer overflow");

        buffer
    }

    let server_dat_file_path = format!("{}\\{}", roaming_dir.display(), MC_SERVER_DAT_PATH);

    let uncompressed_nbt_bytes = get_file_as_byte_vec(&server_dat_file_path);

    let nbt = if uncompressed_nbt_bytes.is_empty() {
        NbtCompound::default()
    } else {
        read_nbt(
            &mut Cursor::new(uncompressed_nbt_bytes.as_slice()),
            Flavor::Uncompressed,
        )
        .unwrap()
        .0
    };

    let nbt_bytes = serialize(&nbt, None, Flavor::Uncompressed).unwrap();

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ServerMetadata {
        icon: Option<String>,
        ip: String,
        name: String,
        accept_textures: Option<i32>,
    }
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct SavedServersList {
        servers: Option<Vec<ServerMetadata>>,
    }

    let mut servers_dat: SavedServersList =
        deserialize(&nbt_bytes, Flavor::Uncompressed).unwrap().0;

    let server_entry = ServerMetadata {
        icon: None,
        ip: SERVER_IP.to_string(),
        name: SERVER_NAME.to_string(),
        accept_textures: None,
    };
    if let Some(mut tmp) = servers_dat.servers {
        if tmp
            .iter()
            .find(|s| s.name == SERVER_NAME.to_string())
            .is_none()
        {
            tmp.push(server_entry);
        }
        servers_dat.servers = Some(tmp);
    } else {
        servers_dat.servers = Some(vec![server_entry]);
    };

    let nbt_bytes = serialize(&servers_dat, None, Flavor::Uncompressed).unwrap();
    let nbt = read_nbt(&mut Cursor::new(&nbt_bytes), Flavor::Uncompressed)
        .unwrap()
        .0;
    let mut f = File::create(&server_dat_file_path).expect("Unable to create file");
    write_nbt(&mut f, None, &nbt, Flavor::Uncompressed).unwrap();
}

fn set_launcher_profile(roaming_dir: PathBuf) {
    let launcher_profile_path = format!("{}\\{}", roaming_dir.display(), MC_LAUNCHER_PROFILE_PATH);
    let mut launcher_profile_json = fs::read_to_string(&launcher_profile_path).unwrap();

    let to_replace = format!("\"name\" : \"{}\"", LAUNCH_PROFILE_NAME);
    launcher_profile_json = launcher_profile_json.replace("\"name\" : \"forge\"", &to_replace);

    let to_replace = format!("\"name\" : \"{}\"", LAUNCH_PROFILE_NAME);
    launcher_profile_json = launcher_profile_json.replace("\"name\": \"forge\"", &to_replace);

    fs::remove_file(&launcher_profile_path).unwrap();
    let mut f =
        File::create(&launcher_profile_path).expect("Unable to open launcher_profiles.json");
    f.write_all(launcher_profile_json.as_bytes()).unwrap();
}
