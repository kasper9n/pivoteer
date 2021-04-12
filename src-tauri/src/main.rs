#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use csv;

mod cmd;

fn handle_cmd(command: cmd::Cmd) -> Result<(), String> {
  use cmd::Cmd::*;
  match command {
    ImportCsv { file_path } => {
      println!("{}", file_path);
      let mut rdr = match csv::Reader::from_path(file_path) {
        Ok(reader) => reader,
        Err(e) => return Err("Error opening csv: ".to_string() + &e.to_string()),
      };
      let mut i = 0;
      for result in rdr.records() {
        let record = match result {
          Ok(record) => record,
          Err(e) => return Err("Error reading record: ".to_string() + &e.to_string()),
        };
        if i == 0 {
          println!("{:?}", record);
          // println!("{}", i);
        }
        i += 1;
      }
      return Ok(());
    }
  }
}

fn main() {
  tauri::AppBuilder::default()
    .invoke_handler(|_webview, arg| -> Result<(), String> {
      let command = match serde_json::from_str(arg) {
        Err(e) => return Err(e.to_string()),
        Ok(command) => command,
      };
      handle_cmd(command)
    })
    .build()
    .run();
}
