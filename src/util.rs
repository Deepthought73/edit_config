use std::collections::HashMap;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use serde_json::{json, Value};
use crate::{Int, List, Object, Str};

pub const EXIT_TEXT: &str = "[ exit ]";
pub const BACK_TEXT: &str = "[ <- back ]";
pub const ADD_ITEM_TEXT: &str = "[ + add item ]";
pub const DELETE_ITEM_TEXT: &str = "[ - delete item ]";

pub const CONFIG_SCHEME_FILENAME: &str = "config_scheme.json";
pub const CONFIG_FILENAME: &str = "config.json";

#[derive(Clone)]
pub enum Scheme {
    Int(i64),
    Str(String),
    Object(HashMap<String, Scheme>),
    List(Box<Scheme>, Value),
}

pub fn load_json_file(filepath: &str) -> Result<Value, String> {
    match File::open(filepath) {
        Ok(mut file) => {
            let mut text = String::new();
            file.read_to_string(&mut text).unwrap();
            match serde_json::from_str::<Value>(&text) {
                Ok(val) => Ok(val),
                Err(err) => Err(format!("The {} is corrupted: {}", filepath, err)),
            }
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Ok(Value::Null),
            _ => Err(format!(
                "Error occurred while reading {}: {}",
                filepath, err
            )),
        },
    }
}

pub fn store_json_file(filepath: &str, value: &Value) {
    let s = serde_json::to_string_pretty(value).unwrap();
    let mut file = File::create(filepath).unwrap();
    file.write(s.as_bytes()).unwrap();
}

pub fn scheme_to_default_value(scheme: &Scheme) -> Value {
    match scheme {
        Int(default) => json!(default),
        Str(default) => json!(default),
        Object(object) => {
            let mut map = HashMap::new();
            for (key, scheme) in object {
                map.insert(key, scheme_to_default_value(scheme));
            }
            json!(map)
        }
        List(_, default) => default.clone()
    }
}

pub fn value_to_scheme(value: Value) -> Result<Scheme, String> {
    let name = value.to_string();
    match value {
        Value::String(t) => match &t[..3] {
            "Str" => Ok(Str(t[3..].trim().to_string())),
            "Int" => Ok(Int(t[3..]
                .trim()
                .parse()
                .map(|it| it)
                .unwrap_or(0))),
            _ => Err(format!("Not allowed type {}", name)),
        },
        Value::Array(l) => match l.get(0) {
            Some(t) => {
                let scheme = value_to_scheme(t.to_owned())?;
                let default = l.get(1)
                    .map(|it| it.clone())
                    .unwrap_or(json!(Vec::<Value>::new()));
                Ok(List(Box::new(scheme), default))
            }
            None => Err(format!("List type has to specified for {}", name)),
        },
        Value::Object(m) => {
            let mut object = HashMap::new();
            for (k, v) in m {
                object.insert(k, value_to_scheme(v)?);
            }
            Ok(Object(object))
        }
        _ => Err("Not accepted scheme type".to_string()),
    }
}

pub fn build_scheme() -> Result<Scheme, String> {
    value_to_scheme(load_json_file(CONFIG_SCHEME_FILENAME)?)
}

pub fn load_config(scheme: &Scheme) -> Result<Value, String> {
    fn contains_scheme(value: &Value, scheme: &Scheme, path: Vec<String>) -> Result<(), String> {
        let error_location = format!("Value [{}]", path.join("]["));
        match scheme {
            Int(_) => if value.is_number() { Ok(()) } else {
                Err(format!("{} has to be a number", error_location))
            },
            Str(_) => if value.is_string() { Ok(()) } else {
                Err(format!("{} has to be a string", error_location))
            },
            Object(object) => {
                if let Some(map) = value.as_object() {
                    for (key, scheme) in object {
                        if let Some(value) = map.get(key) {
                            let mut path = path.clone();
                            path.push(key.to_string());
                            contains_scheme(value, scheme, path)?
                        } else {
                            return Err(format!("{} doesn't has child {}", error_location, key.to_string()));
                        }
                    }
                    Ok(())
                } else {
                    Err(format!("{} has to an object", error_location))
                }
            }
            List(scheme, _) => {
                if let Some(list) = value.as_array() {
                    for (i, value) in list.iter().enumerate() {
                        let mut path = path.clone();
                        path.push(i.to_string());
                        contains_scheme(value, scheme, path)?
                    }
                    Ok(())
                } else {
                    Err(format!("{} has to be an array", error_location))
                }
            }
        }
    }

    let mut value = load_json_file(CONFIG_FILENAME)?;

    if value.is_null() {
        value = scheme_to_default_value(scheme)
    }

    contains_scheme(&value, scheme, vec![]).map(|()| value).map_err(|err| {
        format!("Invalid config file: {}\n\
                     If you delete the config file, a new correct one will be created", err)
    })
}
