use std::fs;
use calamine::{open_workbook, Reader, Xlsx};
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use serde_json::{json, Value};
use crate::BACK_TEXT;

pub fn read_excel_to_drive_list(config: &mut Value) {
    let files = fs::read_dir(".").unwrap();
    let mut xlsx_files = vec![];
    for path in files {
        let path = path.unwrap();
        if path.file_type().unwrap().is_file()
            && path.file_name().to_str().unwrap().ends_with(".xlsx") {
            xlsx_files.push(path.file_name().to_str().unwrap().to_string())
        }
    }

    if xlsx_files.is_empty() {
        println!("There is no Excel (*.xlsx) file that could be imported in this directory.\nMove the desired file to this directory.");
        Select::with_theme(&ColorfulTheme::default())
            .items(&vec![BACK_TEXT])
            .default(0)
            .interact()
            .unwrap();
    } else {
        println!("Select the Excel file you want to import");

        let index = Select::with_theme(&ColorfulTheme::default())
            .items(&xlsx_files)
            .default(0)
            .interact()
            .unwrap();

        let mut workbook: Xlsx<_> = open_workbook(xlsx_files.get(index).unwrap()).unwrap();

        let unit_name_col = 0;
        let unit_nr_col = 2;
        let module_name_col = 8;

        let mut drives = vec![];
        if let Some(Ok(data)) = workbook.worksheet_range("Data") {
            let mut row = 1;
            loop {
                let unit_name = data.get_value((row, unit_name_col));
                let unit_nr = data.get_value((row, unit_nr_col));
                let module_name = data.get_value((row, module_name_col));

                if let (Some(unit_name), Some(unit_nr), Some(module_name)) = (unit_name, unit_nr, module_name) {
                    let mut drive = Value::Object(Default::default());
                    let drive_ = drive.as_object_mut().unwrap();
                    drive_.insert("unit_name".to_string(), json!(unit_name.to_string()));
                    drive_.insert("unit_number".to_string(), json!(unit_nr.to_string()));
                    drive_.insert("module_name".to_string(), json!(module_name.to_string()));
                    drive_.insert("trigger_interval_min".to_string(), json!(180));
                    drive_.insert("trigger_type_scheme".to_string(), json!(vec![1, 2, 3]));

                    drives.push(drive);
                    row += 1
                } else {
                    break;
                }
            }
        }

        config.as_array_mut().unwrap().extend(drives)
    }
}
