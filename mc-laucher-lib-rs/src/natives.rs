use crate::expections::{LauncherLibError, LibResult };
use crate::json::{
    install::{
        ExtractFile,
        Library
    }
};
use std::env::consts;
use std::fs::{ File, create_dir_all };
use std::path::PathBuf;
use std::io::Read;
use log::{ error };

pub fn get_natives(library: &Library) -> String {
    let arch = match consts::ARCH {
        "x86" => "32",  
        _ => "64"
    };

    if let Some(native) = &library.natives {
        let os = match consts::OS {
            "macos" => "osx",
            _ => consts::OS
        };

        if let Some(value) = native.get(os) {
            return value.clone().replace("{$arch}",arch);
        }
    }

    String::default()
}

pub fn extract_natives_file(filename: PathBuf, extract_path: &PathBuf, extract_data: &ExtractFile) -> LibResult<()> {

    if let Err(error) = create_dir_all(extract_path.clone()) {
        return Err(LauncherLibError::OS {
            msg: "Failed to create directory".into(),
            source: error
        });
    }

    let file = match File::open(filename) {
        Err(error) => return Err(LauncherLibError::OS{
            source: error,
            msg: "Failed to open file".into()
        }),
        Ok(value) => value
    };

    let mut zip: zip::ZipArchive<File> = match zip::ZipArchive::new(file) {
        Ok(value) => value,
        Err(error) => return Err(LauncherLibError::ZipError(error))
    };

    let mut ignores: Vec<String> = vec![];

    for file in &extract_data.exclude {
        ignores.push(file.clone());
    }

    for i in 0..zip.len() {
        match zip.by_index(i) {
            Ok(mut item) => {
               let mut skip = false;
               for e in &ignores {
                   if item.name().starts_with(e) {
                       skip = true;
                       break;
                   }
               }
               if skip { continue; }

               let mut buffer: Vec<u8> = vec![];
               if let Err(err) = item.read_to_end(&mut buffer) {
                   return Err(LauncherLibError::OS {
                       msg: "Failed to write file to buffer".into(),
                       source: err
                   });
               };

               if let Err(err) = File::create(extract_path.join(item.name())) {
                    return Err(LauncherLibError::OS {
                        msg: "Failed to write file".into(),
                        source: err
                    });
               }
            }
            Err(err) => error!("{}",LauncherLibError::ZipError(err))
        }
    }
    Ok(())
}

pub fn get_library_data(name: String) -> LibResult<(String,String,String)> {
    let data = name.split(":").collect::<Vec<&str>>();

    if data.len() != 3 {
        return Err(LauncherLibError::General("Library name does not content required params".into()))
    }

    Ok((data[0].into(),data[1].into(),data[2].into()))
} 

// Extract natives into the givrn path.
/*pub fn extract_natives(version_id: String, path: PathBuf, extract_path: PathBuf) -> LibResult<()> {

    let file = path.join("versions").join(version_id.clone()).join(format!("{}.json",version_id));

    if !file.is_file() {
        return Err(LauncherLibError::NotFound(version_id));
    }

    let manifest: VersionManifest = match read_to_string(file) {
        Ok(raw) => {
            match serde_json::from_str::<VersionManifest>(&raw) {
                Ok(value) => value,
                Err(err) => return Err(LauncherLibError::ParseJsonSerde(err))
            }
        }
        Err(err) => return Err(LauncherLibError::OS {
            source: err,
            msg: "Failed to read file".into()
        })
    };

    for i in manifest.libraries {

        if let Some(rules) = &i.rules {
            if !parse_rule_list(&rules, &mut GameOptions::default()) {
                continue;
            }
        }

       let mut current_path = path.join("libraries");

       let (lib_path,name,version) = match get_library_data(i.name.clone()) {
           Ok(value) => value,
           Err(err) => return Err(err)
       };


       for lib_part in lib_path.split(".").collect::<Vec<&str>>() {
            current_path = current_path.join(lib_part);
       }

       current_path = current_path.join(name.clone()).join(version.clone());

       let native = get_natives(&i);

       if native.is_empty() {
           continue;
       }

       let jar_filename_native = format!("{}-{}-{}.jar",name,version,native);

       if let Some(extract) = &i.extract {
            if let Err(err) = extract_natives_file(current_path.join(jar_filename_native),&extract_path,extract) {
                return Err(err);
            }
       }
    }

    Ok(())
}*/