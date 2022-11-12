mod plugin;
mod util;

use crate::Scheme::{Int, List, Object, Str};
use std::io::stdin;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use serde_json::Value;
use std::collections::HashMap;
use std::process::exit;
use crate::plugin::read_excel_to_drive_list;
use crate::util::*;

fn clear_screen() {
    Term::stdout().clear_screen().unwrap();
}

struct Configuration {
    scheme: Scheme,
    config: Value,
    path: Vec<(String, usize)>,
    path_string: String,
    last_selected_index: usize,
    exit: bool,
    delete_mode: bool,
    plugins: HashMap<String, (String, fn(&mut Value))>,
}

impl Configuration {
    fn new(plugins: HashMap<String, (String, fn(&mut Value))>) -> Result<Self, String> {
        let scheme = build_scheme()?;
        let config = load_config(&scheme)?;

        Ok(Self {
            scheme,
            config,
            path: vec![],
            path_string: "[]".to_string(),
            last_selected_index: 0,
            exit: false,
            delete_mode: false,
            plugins,
        })
    }

    fn current_scheme(&self) -> &Scheme {
        let mut current = &self.scheme;
        for (p, _) in &self.path {
            match current {
                Object(object) => current = object.get(p).unwrap(),
                List(list, _) => current = list,
                _ => exit(-1),
            }
        }
        current
    }

    fn current_config_value(&mut self) -> &mut Value {
        let mut current = &mut self.config;
        for (p, _) in &self.path {
            match current {
                Value::Object(object) => current = object.get_mut(p).unwrap(),
                Value::Array(list) => current = list.get_mut(p.parse::<usize>().unwrap()).unwrap(),
                _ => {}
            }
        }
        current
    }

    fn save(&self) {
        store_json_file(CONFIG_FILENAME, &self.config);
        println!("Saved changes!");
    }

    fn exit(&mut self) {
        self.exit = true;

        clear_screen();
        println!("Do you want to save your changes?");
        let index = Select::with_theme(&ColorfulTheme::default())
            .items(&vec!["Save", "Discard"])
            .default(0)
            .interact()
            .unwrap();

        clear_screen();
        if index == 0 {
            self.save()
        } else {
            println!("Discarded changes!");
        }
    }

    fn path_pop(&mut self) {
        self.delete_mode = false;
        self.last_selected_index = self.path.pop().map(|(_, i)| i + 2).unwrap_or(0);
    }

    fn path_push(&mut self, item: (String, usize)) {
        self.delete_mode = false;
        self.last_selected_index = 0;
        self.path.push(item);
    }

    fn configure_int(&mut self) {
        let value = self.current_config_value().as_i64().unwrap();

        #[cfg(target_os = "linux")]
            let new_value: i64 = Input::new()
            .with_prompt("Value")
            .with_initial_text(value.to_string())
            .interact_text()
            .unwrap();
        #[cfg(target_os = "windows")]
        let new_value = {
            print!("Value: {}", value);
            let mut s = String::new();
            stdin().read_line(&mut s).unwrap();
            s.parse().unwrap()
        };

        *self.current_config_value() = Value::Number(new_value.into());
        self.path_pop()
    }

    fn configure_str(&mut self) {
        let value = self.current_config_value().as_str().unwrap();

        let new_value: String = Input::new()
            .with_prompt("Value")
            .with_initial_text(value)
            .allow_empty(true)
            .interact_text()
            .unwrap();

        *self.current_config_value() = Value::String(new_value.trim().to_string());
        self.path_pop()
    }

    fn show_items(&mut self, items: &mut Vec<String>) -> Option<usize> {
        items.insert(0, EXIT_TEXT.to_string());
        if !self.path.is_empty() {
            items.insert(0, BACK_TEXT.to_string());
        } else if self.last_selected_index > 0 {
            self.last_selected_index -= 1;
        }

        let mut active_plugins = vec![];
        if !self.delete_mode {
            if let Some((name, plugin)) = self.plugins.iter()
                .find(|(key, _)| self.path_string.ends_with(*key))
                .map(|(_, it)| it) {
                items.push(name.clone());
                active_plugins.push(plugin.clone())
            }
        }

        let index = Select::with_theme(&ColorfulTheme::default())
            .items(&items)
            .default(self.last_selected_index)
            .interact()
            .unwrap();

        let last_index = items.len();
        if index == 0 {
            if self.path.is_empty() {
                self.exit();
            } else {
                self.path_pop()
            }
        } else if !self.path.is_empty() && index == 1 {
            self.exit();
        } else if index >= last_index - active_plugins.len() {
            active_plugins.get(index - (last_index - active_plugins.len()))
                .unwrap()(self.current_config_value());
        } else {
            return if self.path.is_empty() {
                Some(index - 1)
            } else {
                Some(index - 2)
            };
        }
        None
    }

    fn configure_object(&mut self, object: &HashMap<String, Scheme>) {
        let config = self.current_config_value().as_object().unwrap();

        let mut items: Vec<String> = object
            .keys()
            .cloned()
            .map(|it| {
                match config.get(&it).unwrap() {
                    Value::Number(val) => format!("{}: {}", it, val),
                    Value::String(val) => format!("{}: \"{}\"", it, val),
                    Value::Array(list) => format!("{} (Size: {})", it, list.len()),
                    Value::Object(_) => it,
                    _ => it,
                }
            })
            .collect();

        if let Some(index) = self.show_items(&mut items) {
            let keys = object.keys().collect::<Vec<_>>();
            let key = keys.get(index).unwrap();
            self.path_push((key.to_string(), index))
        }
    }

    fn configure_list(&mut self, default_value: &Value) {
        let delete_mode = self.delete_mode;
        let config = self.current_config_value().as_array().unwrap();

        let mut items: Vec<String> = config
            .iter()
            .map(|it| {
                let mut s = if delete_mode {
                    format!("-> {}", it)
                } else {
                    format!("{}", it)
                };
                if s.len() > 100 {
                    s = s[..100].to_string();
                    s.push_str("...")
                }
                s
            })
            .collect();

        let items_empty = items.is_empty();

        if !self.delete_mode {
            if !items_empty {
                items.push(DELETE_ITEM_TEXT.to_string());
            }
            items.push(ADD_ITEM_TEXT.to_string());
        } else {
            println!("Select the item to be deleted")
        }

        let last_index = items.len() - 1;
        if let Some(index) = self.show_items(&mut items) {
            if self.delete_mode {
                let config = self.current_config_value().as_array_mut().unwrap();
                config.remove(index);
                self.delete_mode = false;
            } else {
                if !items_empty && index == last_index - 1 {
                    self.last_selected_index = 0;
                    self.delete_mode = true
                } else if index == last_index {
                    let config = self.current_config_value().as_array_mut().unwrap();
                    let empty = config.is_empty();
                    config.push(default_value.clone());
                    let len = config.len();
                    self.last_selected_index = if empty {
                        last_index + 2
                    } else {
                        last_index + 1
                    };
                    self.path_push(((len - 1).to_string(), len - 1))
                } else {
                    self.path_push((index.to_string(), index));
                }
            }
        }
    }

    fn configure(&mut self) {
        while !self.exit {
            clear_screen();
            self.path_string = "[".to_string()
                + &self.path.iter()
                .map(|(it, _)| it.clone())
                .collect::<Vec<String>>().join("][")
                + "]";
            println!("Path:  {}", self.path_string);

            match self.current_scheme().clone() {
                Int(_) => self.configure_int(),
                Str(_) => self.configure_str(),
                Object(object) => self.configure_object(&object),
                List(default_scheme, _) => self.configure_list(
                    &scheme_to_default_value(&default_scheme)
                )
            }
        }
    }
}

fn main() {
    let mut plugins: HashMap<String, (String, fn(&mut Value))> = HashMap::new();
    plugins.insert("[drives]".to_string(), ("[ load drives from Excel ]".to_string(), read_excel_to_drive_list));

    match Configuration::new(plugins) {
        Ok(mut config) => config.configure(),
        Err(err) => println!("{}", err),
    };
}
