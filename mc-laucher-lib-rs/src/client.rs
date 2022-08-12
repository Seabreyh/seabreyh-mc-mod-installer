use crate::command::get_launch_command;
use crate::expections::{LauncherLibError,LibResult};
use crate::utils::get_minecraft_directory;
use crate::install::install_minecraft_version;
use crate::fabric::install_fabric;
use crate::forge::install_forge;
use crate::optifine::install_optifine;
use crate::json::{
    game_settings::{ 
        GameOptions,
        UserType
    },
    authentication_microsoft::Account,
    install::Callback,
    client::{ InstallManifest, Loader }
};
use std::path::PathBuf;
use uuid::Uuid;
use std::process::{ Command, Child};

#[derive(Debug, Default)]
pub struct Client {
    options: GameOptions,
    minecraft: String,
    minecraft_directory: PathBuf,
    process: Option<Child> 
}
impl Client {
    pub fn new(minecraft: String, minecraft_directory: PathBuf, options: GameOptions) -> Self {
        Self {
            options,
            minecraft,
            minecraft_directory,
            process: None
        }
    }
    pub fn is_running(&mut self) -> LibResult<bool> {
        if let Some(p) = &mut self.process {
            match p.try_wait() {
                Ok(Some(_status)) => {
                    self.process = None;
                    Ok(false)
                },
                Ok(None) => Ok(true),
                Err(err) => Err(LauncherLibError::General(err.to_string()))
            }
        } else {
            Ok(false)
        }
    }
    pub async fn start(&mut self) -> LibResult<()> {
        if self.process.is_some() {
            return Err(LauncherLibError::General("A instance of minecraft is already running".into()));
        }

        let (java,args) = match get_launch_command(self.minecraft.clone(), self.minecraft_directory.clone(), &mut self.options).await {
            Ok(value) => value,
            Err(err) => return Err(err)
        };

        let handler: Child = match Command::new(java).args(args).spawn() {
            Err(err) => return Err(LauncherLibError::OS {
                source: err,
                msg: "Failed to launch minecraft".into()
            }),
            Ok(handle) => handle
        };


        self.process = Some(handler);

        Ok(())
    }
    pub fn exit(&mut self) -> LibResult<()> {
        if let Some(process) = &mut self.process {
            match process.wait() {
                Err(err) => return Err(LauncherLibError::OS { 
                    source: err,
                    msg: "Minecraft was not running".into()
                }),
                Ok(_) => { self.process = None; }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ClientBuilder {
    options: GameOptions,
    minecraft: String,
    minecraft_directory: PathBuf 
}

impl ClientBuilder {
    pub async fn install(manifest: InstallManifest, minecraft_directory: Option<PathBuf>, callback: Callback, temp_path: Option<PathBuf>, cache_path: Option<PathBuf>, java: Option<PathBuf>) -> LibResult<()> {
        let mc_dir = if let Some(dir) = minecraft_directory { dir } else { 
            match get_minecraft_directory() {
                Ok(value) => value,
                Err(err) => return Err(err)
            } 
        };

        match manifest.modloader.clone() {
            Loader::Fabric => {
                let temp = match temp_path {
                    Some(value) => value,
                    None => return Err(LauncherLibError::General("Missing temp path".into()))
                };
                install_fabric(manifest.minecraft.clone(),mc_dir,manifest.modloader_version,callback,java,temp).await
            }
            Loader::Forge => {
                let temp = match temp_path {
                    Some(value) => value,
                    None => return Err(LauncherLibError::General("Missing temp path".into()))
                };
                install_forge(manifest.minecraft.clone(), mc_dir, temp, callback, cache_path, manifest.modloader_version, java, manifest.cache_cli, manifest.cache_install).await
            }
            Loader::Optifine => {
                let temp = match temp_path {
                    Some(value) => value,
                    None => return Err(LauncherLibError::General("Missing temp path".into()))
                };
                install_optifine(manifest.minecraft.clone(), mc_dir, temp, callback, cache_path, manifest.modloader_version, java, manifest.cache_cli, manifest.cache_install).await
            }
            Loader::Vanilla => {
                install_minecraft_version(manifest.minecraft.clone(),mc_dir,callback).await
            }
        }
    }
    pub async fn install_str(manifest: String, minecraft_directory: Option<PathBuf>, callback: Callback, temp_path: Option<PathBuf>, cache_path: Option<PathBuf>, java: Option<PathBuf>) -> LibResult<()> {
        match serde_json::from_str::<InstallManifest>(&manifest) {
            Err(err) => Err(LauncherLibError::ParseJsonSerde(err)),
            Ok(value) => ClientBuilder::install(value, minecraft_directory, callback, temp_path, cache_path, java).await
        }
    }
    pub fn new(minecraft_dir: Option<PathBuf>) -> LibResult<Self> {
        let mc_dir = match minecraft_dir {
            Some(value) => value,
            None => {
                match get_minecraft_directory() {
                    Ok(value) => value,
                    Err(err) => return Err(err)
                }
            }
        };

        Ok(Self {
            minecraft_directory: mc_dir,
            ..Default::default()
        })
    }
    pub fn set_jvm_args(&mut self, args: String) -> &mut Self {
        self.options.jvm_arguments = Some(args);
        self
    }
    pub fn set_java(&mut self, java: Option<PathBuf>) -> &mut Self {
        if let Some(path) = java {
            self.options.executable_path = Some(path);
        }
        self
    }
    pub fn as_dev_user(&mut self) -> &mut Self {
        self.options.uuid = Some(Uuid::new_v4().to_hyphenated().to_string());
        self.options.username = Some("Rusty".into());
        self.options.user_type = UserType::Unkown;
        self
    }
    pub fn as_user(&mut self, username: String, uuid: String, token: String) -> &mut Self {
        self.options.uuid = Some(uuid);
        self.options.username = Some(username);
        self.options.token = Some(token);
        self.options.user_type = UserType::Mojang;
        self
    }
    /// similar to set_user but data is set when user is loggin
    pub fn as_msa_user(&mut self, user: Account) -> &mut Self {
        self.options.user_type = UserType::Microsoft;
        self.options.xuid = Some(user.xuid.clone());
        self.options.token = Some(user.access_token.clone());
        self.options.uuid = Some(user.profile.id.clone());
        self.options.username = Some(user.profile.name.clone());
    
        self
    }
    pub fn set_client_id(&mut self, id: String) -> &mut Self {
        self.options.client_id = Some(id);
        self
    }
    pub fn enable_logging(&mut self) -> &mut Self {

        self.options.enable_logging_config = true;
        
        self
    }
    pub fn set_minecraft(&mut self, minecraft: String, loader: Option<Loader>, loader_version: Option<String>) -> LibResult<&mut Self> {
        match loader {
            Some(value) => {
                match value {
                    Loader::Fabric => {
                        if let Some(lv) = loader_version {
                            self.minecraft = format!("fabric-loader-{loader}-{mc}", loader=lv,mc=minecraft).to_string();
                        } else {
                            return Err(LauncherLibError::General("Missing loader version".into()))
                        }
                    },
                    Loader::Forge => {
                        if let Some(lv) = loader_version {
                            self.minecraft = format!("{mc}-forge-{loader}", loader=lv,mc=minecraft).to_string();
                        } else {
                            return Err(LauncherLibError::General("Missing loader version".into()))
                        }
                    },
                    Loader::Optifine => {
                        if let Some(lv) = loader_version {
                            self.minecraft = format!("{mc}-OptiFine_{loader}", loader=lv,mc=minecraft).to_string();
                        } else {
                            return Err(LauncherLibError::General("Missing loader version".into()))
                        }
                    }
                   _ => {
                        self.minecraft = minecraft;
                   }
                }
            }
            None => {
                self.minecraft = minecraft;
            }
        }
        Ok(self)
    }
    pub fn build(&self) -> LibResult<Client> {

        if self.minecraft.is_empty() {
            return Err(LauncherLibError::General("Minecraft version is unset".into()));
        }
        // should do some checks here like,
        // setting of the minecraft version

        Ok(Client::new(self.minecraft.clone(),self.minecraft_directory.clone(),self.options.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time;

    #[tokio::test]
    async fn test_optifine_install() {
        if let Err(err) = ClientBuilder::install(
                InstallManifest::new("1.18.1".into(), Loader::Optifine), 
                None,
                |event| { println!("{:#?}",event); },
                Some(PathBuf::from("C:\\Users\\Collin\\Downloads\\")), None, None
            ).await {
            eprintln!("{}",err);
        }
    }

    #[tokio::test]
    async fn test_forge_install() {
        if let Err(err) = ClientBuilder::install(
                InstallManifest::new("1.18.1".into(), Loader::Forge), 
                None,
                |event| { println!("{:#?}",event); },
                Some(PathBuf::from("C:\\Users\\Collin\\Downloads\\")), None, None
            ).await {
            eprintln!("{}",err);
        }
    }

    #[tokio::test]
    async fn test_fabric_install() {
        if let Err(err) = ClientBuilder::install(
            InstallManifest::new("1.18.1".into(), Loader::Fabric), 
                None,
                |event| { println!("{:#?}",event); },
                Some(PathBuf::from("C:\\Users\\Collin\\Downloads\\")), None, None
            ).await {
            eprintln!("{}",err);
        }
    }

    #[tokio::test]
    async fn test_vinilla_install() {
        if let Err(err) = ClientBuilder::install(
            InstallManifest::new("1.17.1".into(), Loader::Vanilla), 
                None, 
                |event| { println!("{:#?}",event); }, None, None, None
            ).await {
            eprintln!("{}",err);
        }
    }

    #[tokio::test]
    async fn test_client_builder() {
        use crate::login::{ms_login_url,get_auth_code,login_microsoft};

        let client_id = std::env::var("CLIENT_ID").expect("Failed to get client id");
        let redirect: String = "https://login.microsoftonline.com/common/oauth2/nativeclient".into();

        let url = ms_login_url(client_id.clone(), redirect.clone());

        println!("{}",url);

        println!("Enter auth");

        let mut raw_auth_code = String::default();
        if let Err(err) = std::io::stdin().read_line(&mut raw_auth_code) {
            eprintln!("{}",err);
            panic!();
        }

        let auth_code = match get_auth_code(raw_auth_code) {
            Some(value)=> value,
            None => {
                eprintln!("Failed to get auth code from url");
                panic!();
            }
        };

        let account = match login_microsoft(client_id.clone(), redirect, auth_code).await {
            Ok(account) => account,
            Err(err) => {
                eprintln!("{}",err);
                panic!();
            }
        };

        match ClientBuilder::new(None) {
            Ok(mut client) => {
                let mut runner = match client
                .set_client_id(client_id)
                .as_msa_user(account)
                .set_jvm_args("-Xmx2G -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M".into())
                .set_minecraft("1.18.1".into(),Some(Loader::Optifine),Some("HD_U_H4".into()))
                .expect("Failed to set mc ver").build() {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!("{}",err);
                        panic!();
                    }
                };

                println!("{:#?}",runner);

                if let Err(err) = runner.start().await {
                    eprintln!("{}",err);
                }

                thread::sleep(time::Duration::from_secs(10));

               /* if let Err(err)  = runner.exit() {
                    eprintln!("{}",err);
                }*/
            }
            Err(err) => eprintln!("{}",err)
        }
    }

}