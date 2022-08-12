use crate::utils::{ parse_rule_list, download_file, read_manifest_inherit };
use crate::vanilla::get_vanilla_versions;
use crate::natives::{ extract_natives_file, get_library_data, get_natives };
use crate::expections::{ LauncherLibError, LibResult };
use crate::runtime::{ install_jvm_runtime, does_runtime_exist };
use crate::json::{
    launcher_version::VersionsManifestVersion,
    game_settings::GameOptions,
    install::{Library,VersionManifest,Callback, Event}
};
use std::path::PathBuf;
use tokio::fs::read_to_string;
use log::{ error };
use serde::Deserialize;

async fn install_libraries(id: String, libraries: &Vec<Library>, path: PathBuf, callback: Callback ) -> LibResult<()> {

    let max = libraries.len();
    
    for (count, i) in libraries.iter().enumerate() {
        if let Some(rules) = &i.rules {
            if !parse_rule_list(&rules, &mut GameOptions::default()) {
                callback(Event::progress(count, max));
                continue;
            }
        }

        let mut current_path = path.join("libraries");

        let mut download_url: String =  if let Some(url) = &i.url {
            if url.ends_with("/") {
                match url.get(0..(url.len()-1)) {
                    Some(uri) => uri.into(),
                    None => return Err(LauncherLibError::General("Failed to remove char / on library url".into()))
                }
            } else {
                url.clone()
            }
        } else {
            "https://libraries.minecraft.net".into()
        };

        let (lib_path,name,version) = match get_library_data(i.name.clone()) {
            Ok(value) => value,
            Err(err) => {
                error!("{}",err);
                continue;
            }
        };

        for lib_part in lib_path.split(".").collect::<Vec<&str>>() {
            current_path = current_path.join(lib_part);
            download_url = format!("{}/{}",download_url,lib_part).to_string();
        }
        
        let version_at = version.split("@").collect::<Vec<&str>>();

        let (version_lib,fileend) = if version_at.len() == 2 {
            (version_at[0],version_at[1])
        } else {
            (version.as_str(),"jar")
        };

        let jar_filename = format!("{}-{}.{}",name,version_lib,fileend).to_string();

        download_url = format!("{}/{}/{}",download_url,name,version_lib).to_string();

        current_path = current_path.join(name.clone()).join(version_lib);

        let native = get_natives(&i);

        let jar_filename_native = if !native.is_empty() {
            format!("{}-{}-{}.jar",name,version,native).to_string()
        } else {
            String::default()
        };

        download_url = format!("{}/{}",download_url,jar_filename).to_string();

        if let Err(err) = download_file(download_url, current_path.join(jar_filename.clone()), callback, None,false).await {
            error!("{}",err);
        }

        if i.downloads.is_none() {
            if let Some(extract) = &i.extract {
                if let Err(err) = extract_natives_file(current_path.join(jar_filename_native), &path.join("versions").join(id.clone()).join("natives"), &extract) {
                    return Err(err);
                }
                continue;
            }
        } 

        if let Some(downloads) = &i.downloads {

            if let Err(err) = download_file(downloads.artifact.url.clone(), current_path.join(jar_filename), callback,Some(downloads.artifact.sha1.clone()), false).await {
                return Err(err);
            }

            if !native.is_empty() {
                if let Some(classifiers) = &downloads.classifiers {
                    if let Some(nat) = classifiers.get(&native) {
                        if let Err(err) = download_file(nat.url.clone(), current_path.join(jar_filename_native.clone()), callback, Some(nat.sha1.clone()), false).await {
                            return Err(err);
                        }
                    }
                    if let Some(extract) = &i.extract {
                        if let Err(err) = extract_natives_file(current_path.join(jar_filename_native), &path.join("versions").join(id.clone()).join("natives"), &extract) {
                            return Err(err);
                        }
                    }
                }
            }
        }
        callback(Event::progress(count,max));
    }
    Ok(())
}

#[derive(Deserialize)]
struct IndexAssetsItem {
    hash: String,
    //size: usize
}
#[derive(Deserialize)]
struct IndexAssetsMap {
    objects: std::collections::HashMap<String,IndexAssetsItem>
}

async fn install_assets(manifest: &VersionManifest, path: PathBuf, callback: Callback) -> LibResult<()> {

    let assets = match &manifest.assets {
        Some(value) => value,
        None => return Err(LauncherLibError::General("Assets key in manifest is missing".into()))
    };

    let index_path = path.join("assets").join("indexes").join(format!("{}.json",assets));
    if let Some(asset_index) = &manifest.asset_index {
        if let Err(err) = download_file(asset_index.url.clone(), index_path.clone(), callback, Some(asset_index.sha1.clone()), false).await {
            return Err(err);
        }

        let assets: IndexAssetsMap = match read_to_string(index_path).await {
            Ok(raw) => {
                match serde_json::from_str::<IndexAssetsMap>(&raw) {
                    Ok(value) => value,
                    Err(err) => return Err(LauncherLibError::ParseJsonSerde(err))
                }
            }
            Err(err) => return Err(LauncherLibError::OS {
                source: err,
                msg: "Failed to read file".into()
            })
        };

        let max = assets.objects.len();
        let mut count = 0;
        for (key, value) in assets.objects {
            callback(Event::Status(format!("Asset: {}",key)));
            let pre = value.hash.get(0..2).expect("Should have this value");
            let url = format!("https://resources.download.minecraft.net/{}/{}",pre,value.hash.clone());
            let outpath = path.join("assets").join("objects").join(pre).join(value.hash.clone());
            if let Err(err) = download_file(url, outpath, callback, Some(value.hash.clone()), false).await {
                return Err(err);
            }
            count += 1;
            callback(Event::progress(count,max));
        }
    } 

    Ok(())
}

async fn do_version_install(version_id: String, path: PathBuf, callback: Callback, url: Option<String>) -> LibResult<()> {

   
    let version_manifest = path.join("versions").join(version_id.clone()).join(format!("{}.json",version_id.clone()));
    callback(Event::Status("Getting version.json file".into()));
    if let Some(url_d) = url {
        if let Err(err) = download_file(url_d, version_manifest.clone(), callback, None, false).await {
            return Err(err);
        }
    }

    
    let manifest: VersionManifest = match read_manifest_inherit(version_manifest,&path).await {
        Ok(value) => value,
        Err(err) => return Err(err)
    };

    callback(Event::Status("Installing libraries".into()));
    if let Err(err) = install_libraries(manifest.id.clone(), &manifest.libraries, path.clone(), callback).await {
        return Err(err);
    }

    callback(Event::Status("Installing Assets".into()));
    if let Err(err) = install_assets(&manifest, path.clone(), callback).await {
        return Err(err);
    }   



    if let Some(logging) = manifest.logging {
        callback(Event::Status("Setting up logging".into()));
        if let Some(client) = logging.get("client") {
            if let Some(id) = &client.file.id {
                let logging_file = path.join("assets").join("log_configs").join(id);
                if let Err(err) = download_file(client.file.url.clone(), logging_file, callback, Some(client.file.sha1.clone()), false).await {
                    return Err(err);
                }
            }
        }
    }

    if let Some(downloads) = manifest.downloads {
        callback(Event::Status("Installing downloads".into()));
        if let Some(client) = downloads.get("client") {
            if let Err(err) = download_file(client.url.clone(), path.join("versions").join(manifest.id.clone()).join(format!("{}.jar",manifest.id.clone())), callback, Some(client.sha1.clone()), false).await {
                return Err(err);
            }
        }
    }

    if let Some(java) = manifest.java_version {
        callback(Event::Status("Installing java runtime".into()));
        match does_runtime_exist(java.component.clone(), path.clone()) {
            Ok(value) => {
                if !value {
                    if let Err(err) = install_jvm_runtime(java.component, path, callback).await {
                        return Err(err);
                    }
                }
            }
            Err(err) => return Err(err)
        }
    }
    Ok(())
}

pub async fn install_minecraft_version(version_id: String, mc_dir: PathBuf, callback: Callback) -> LibResult<()> {
    if mc_dir.join("versions").join(version_id.clone()).join(format!("{}.json",version_id)).is_file() {
       return do_version_install(version_id, mc_dir, callback, None).await;
    }
    match get_vanilla_versions().await {
        Ok(versions) => {

            let version: Vec<&VersionsManifestVersion> = versions.iter().filter(| item | item.id == version_id).collect();

            if let Some(item) = version.get(0) {
                return do_version_install(item.id.clone(), mc_dir, callback, Some(item.url.clone())).await;
            }

            Ok(())
        }
        Err(err) => Err(err)
    }
}