mod ferium;

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{fs::File, io::Cursor};

use iced::window::Icon;

use iced_native::Runtime;
use image::{EncodableLayout, GenericImageView};
use mc_laucher_lib_rs::{
    client::ClientBuilder,
    json::client::{InstallManifest, Loader},
};
use quartz_nbt::io::{read_nbt, write_nbt, Flavor};
use quartz_nbt::serde::{deserialize, serialize};
use quartz_nbt::NbtCompound;

use serde::{Deserialize, Serialize};

const MODS_OUTPUT_DIR: &str = ".minecraft\\mods";
const TMP_MOD_DOWNLOAD_DIR: &str = "Downloads\\";
const FERIUM_PROFILE_NAME: &str = "seabreyh_mods";
const GAME_VERSION: &str = "1.18.2";
const MOD_LOADER: &str = "Forge";
const MC_SERVER_DAT_PATH: &str = ".minecraft\\servers.dat";
const MC_LAUNCHER_PROFILE_PATH: &str = ".minecraft\\launcher_profiles.json";
const LAUNCH_PROFILE_NAME: &str = "Seabreyh Mods";

const SERVER_NAME: &str = "Seabreyh MC Server";
const SERVER_IP: &str = "mc.seabreyh.com";

use iced::{
    alignment, button, executor, futures, window, Alignment, Application, Button, Column, Command,
    Container, Element, Length, ProgressBar, Row, Settings, Text,
};

async fn run_install(user_path: PathBuf, roaming_path: PathBuf) {
    let mods_out_dir = format!("{}\\{}", roaming_path.display(), MODS_OUTPUT_DIR);
    ferium::create_config(&mods_out_dir, FERIUM_PROFILE_NAME, GAME_VERSION, MOD_LOADER);
    ferium::install_mods();
    add_server_to_client(roaming_path.clone());
    install_forge_client(user_path).await;
    set_launcher_profile(roaming_path);
    println!("Install complete!");
}

#[tokio::main]
pub async fn main() -> iced::Result {
    let icon_bytes = include_bytes!("..\\icon.png");
    let image = image::load_from_memory(icon_bytes).expect("Could not load icon");
    let rgba = image.to_rgba8();
    let dimensions = image.dimensions();
    let icon = Icon::from_rgba(rgba.as_bytes().to_vec(), dimensions.0, dimensions.1).ok();

    InstallerApp::run(Settings {
        window: window::Settings {
            size: (384, 200),
            icon,
            ..Default::default()
        },
        ..Default::default()
    })
}

struct InstallerApp {
    state: State,
    last_state: State,
    install_button: button::State,
    progress_bar: f32,
    user_dir: PathBuf,
    roaming_dir: PathBuf,
}

#[derive(Copy, Clone)]
enum State {
    Idle,
    Installing,
    Finished,
}

#[derive(Debug, Clone)]
enum Message {
    Install,
    Finished,
}

impl Application for InstallerApp {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (InstallerApp, Command<Message>) {
        (
            InstallerApp {
                state: State::Idle,
                last_state: State::Idle,
                install_button: button::State::new(),
                progress_bar: 0.0,
                user_dir: dirs::home_dir().unwrap(),
                roaming_dir: dirs::config_dir().unwrap(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Seabreyh Minecraft Mods Installer")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Install => match self.state {
                State::Idle => {
                    self.state = State::Installing;
                }
                _ => {}
            },
            Message::Finished => {
                self.state = State::Finished;
                self.progress_bar = 100.0;
            }
        }

        let mut command = Command::none();
        if let State::Installing = self.state {
            if let State::Idle = self.last_state {
                self.progress_bar = 50.0;
                let user_dir = self.user_dir.clone();
                let roaming_dir = self.roaming_dir.clone();
                command = Command::perform(run_install(user_dir, roaming_dir), move |_| {
                    Message::Finished
                });
            }
        }

        self.last_state = self.state;

        command
    }

    fn view(&mut self) -> Element<Message> {
        let info_text = Text::new(format!(
            "Install Minecraft Forge and Seabreyh's server mods",
        ))
        .size(20);

        let button = |state, label, style| {
            Button::new(
                state,
                Text::new(label).horizontal_alignment(alignment::Horizontal::Center),
            )
            .padding(10)
            .width(Length::Units(80))
            .style(style)
        };

        let install_button = {
            let (label, color) = match self.state {
                State::Idle => ("Install", style::Button::Primary),
                State::Installing => ("Installing...", style::Button::Secondary),
                State::Finished => ("Finished", style::Button::Secondary),
            };

            button(&mut self.install_button, label, color)
                .width(Length::Units(200))
                .on_press(Message::Install)
        };

        let controls = Row::new().spacing(20).push(install_button);

        let content = Column::new()
            .align_items(Alignment::Center)
            .spacing(20)
            .push(info_text)
            .push(controls)
            .push(ProgressBar::new(0.0..=100.0, self.progress_bar).width(Length::Units(350)));

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

mod style {
    use iced::{button, Background, Color, Vector};
    pub enum Button {
        Primary,
        Secondary,
    }

    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: Some(Background::Color(match self {
                    Button::Primary => Color::from_rgb(0.11, 0.42, 0.87),
                    Button::Secondary => Color::from_rgb(0.5, 0.5, 0.5),
                })),
                border_radius: 12.0,
                shadow_offset: Vector::new(1.0, 1.0),
                text_color: Color::WHITE,
                ..button::Style::default()
            }
        }
    }
}

async fn install_forge_client(user_path: PathBuf) {
    if let Err(_) = ClientBuilder::install(
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
    {}
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
    if let Some(mut tmp) = servers_dat.servers.take() {
        tmp.push(server_entry);
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
