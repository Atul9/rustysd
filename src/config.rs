use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use toml;

#[derive(Debug)]
pub struct LoggingConfig {
    pub log_dir: PathBuf,
}

#[derive(Debug)]
pub struct Config {
    pub unit_dirs: Vec<PathBuf>,
    pub notification_sockets_dir: PathBuf,
}

#[derive(Debug)]
enum SettingValue {
    Str(String),
    Array(Vec<SettingValue>),
}

fn load_toml(
    config_path: &PathBuf,
    settings: &mut HashMap<String, SettingValue>,
) -> Result<(), String> {
    let toml_conf: toml::Value = match File::open(&config_path) {
        Ok(mut file) => {
            let mut config = String::new();
            use std::io::Read;
            file.read_to_string(&mut config).unwrap();

            toml::from_str(&config).map_err(|e| format!("Error while decoding config json: {}", e))
        }
        Err(e) => Err(format!("Error while opening config file: {}", e)),
    }?;

    if let toml::Value::Table(map) = &toml_conf {
        if let Some(toml::Value::Array(elems)) = map.get("unit_dirs") {
            settings.insert(
                "unit.dirs".to_owned(),
                SettingValue::Array(
                    elems
                        .into_iter()
                        .map(|e| {
                            if let toml::Value::String(s) = e {
                                SettingValue::Str(s.clone())
                            } else {
                                SettingValue::Str("".to_owned())
                            }
                        })
                        .collect(),
                ),
            );
        }

        if let Some(toml::Value::String(val)) = map.get("logging_dir") {
            settings.insert("logging.dir".to_owned(), SettingValue::Str(val.clone()));
        }
        if let Some(toml::Value::String(val)) = map.get("notifications_dir") {
            settings.insert(
                "notifications.dir".to_owned(),
                SettingValue::Str(val.clone()),
            );
        }
    }
    Ok(())
}

fn load_json(
    config_path: &PathBuf,
    settings: &mut HashMap<String, SettingValue>,
) -> Result<(), String> {
    let json_conf: serde_json::Value = match File::open(config_path) {
        Ok(mut file) => serde_json::from_reader(&mut file)
            .map_err(|e| format!("Error while decoding config json: {}", e)),
        Err(e) => Err(format!("Error while opening config file: {}", e)),
    }?;

    if let serde_json::Value::Object(map) = &json_conf {
        if let Some(serde_json::Value::Array(elems)) = map.get("unit_dirs") {
            settings.insert(
                "unit.dirs".to_owned(),
                SettingValue::Array(
                    elems
                        .into_iter()
                        .map(|e| {
                            if let serde_json::Value::String(s) = e {
                                SettingValue::Str(s.clone())
                            } else {
                                SettingValue::Str("".to_owned())
                            }
                        })
                        .collect(),
                ),
            );
        }

        if let Some(serde_json::Value::String(val)) = map.get("logging_dir") {
            settings.insert("logging.dir".to_owned(), SettingValue::Str(val.clone()));
        }
        if let Some(serde_json::Value::String(val)) = map.get("notifications_dir") {
            settings.insert(
                "notifications.dir".to_owned(),
                SettingValue::Str(val.clone()),
            );
        }
    }
    Ok(())
}

pub fn load_config(config_path: Option<&PathBuf>) -> (LoggingConfig, Result<Config, String>) {
    let mut settings: HashMap<String, SettingValue> = HashMap::new();

    let default_config_path_json = PathBuf::from("./config/rustysd_config.json");
    let default_config_path_toml = PathBuf::from("./config/rustysd_config.toml");

    let config_path_json = if let Some(config_path) = config_path {
        config_path.join("rustysd_config.json")
    } else {
        default_config_path_json.clone()
    };

    let config_path_toml = if let Some(config_path) = config_path {
        config_path.join("rustysd_config.toml")
    } else {
        default_config_path_toml.clone()
    };

    let json_conf = if config_path_json.exists() {
        Some(load_json(&config_path_json, &mut settings))
    } else {
        None
    };

    let toml_conf = if config_path_toml.exists() {
        Some(load_toml(&config_path_toml, &mut settings))
    } else {
        None
    };

    std::env::vars().for_each(|(key, value)| {
        let mut new_key: Vec<String> = key.split('_').map(|part| part.to_lowercase()).collect();
        //drop prefix
        if *new_key[0] == *"rustysd" {
            new_key.remove(0);
            let new_key = new_key.join(".");
            settings.insert(new_key, SettingValue::Str(value.into()));
        }
    });

    let log_dir = settings.get("logging.dir").map(|dir| match dir {
        SettingValue::Str(s) => Some(PathBuf::from(s)),
        _ => None,
    });

    let notification_sockets_dir = settings.get("notifications.dir").map(|dir| match dir {
        SettingValue::Str(s) => Some(PathBuf::from(s)),
        _ => None,
    });

    let unit_dirs = settings.get("unit.dirs").map(|dir| match dir {
        SettingValue::Str(s) => vec![PathBuf::from(s)],
        SettingValue::Array(arr) => arr
            .iter()
            .map(|el| match el {
                SettingValue::Str(s) => {
                    println!("s: {}", s);
                    Some(PathBuf::from(s))
                }
                _ => None,
            })
            .fold(Vec::new(), |mut acc, el| {
                if let Some(path) = el {
                    println!("Got none");
                    if path.exists() {
                        acc.push(path)
                    }
                }
                acc
            }),
    });

    println!("Settings: {:?}", unit_dirs);

    let config = Config {
        unit_dirs: unit_dirs.unwrap_or_else(|| vec![PathBuf::from("./test_units")]),

        notification_sockets_dir: notification_sockets_dir
            .unwrap_or_else(|| Some(PathBuf::from("./notifications")))
            .unwrap_or_else(|| PathBuf::from("./notifications")),
    };

    let conf = if let Some(json_conf) = json_conf {
        if toml_conf.is_some() {
            Err(format!("Found both json and toml conf!"))
        } else {
            match json_conf {
                Err(e) => Err(e),
                Ok(_) => Ok(config),
            }
        }
    } else {
        match toml_conf {
            Some(Err(e)) => Err(e),
            Some(Ok(_)) => Ok(config),
            None => {
                if *config_path_toml == default_config_path_toml {
                    Ok(config)
                } else {
                    Err("No config file was loaded".into())
                }
            }
        }
    };

    (
        LoggingConfig {
            log_dir: log_dir
                .unwrap_or_else(|| Some(PathBuf::from("./logs")))
                .unwrap_or_else(|| PathBuf::from("./logs")),
        },
        conf,
    )
}
