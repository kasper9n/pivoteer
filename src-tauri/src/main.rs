#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::thread;
use tauri::api::{dialog, shell};
use tauri::{
  command, AboutMetadata, AppHandle, CustomMenuItem, Manager, Menu, MenuEntry, MenuItem, State,
  Submenu, Window, WindowBuilder, WindowUrl,
};
use typescript_type_def::TypeDef;

mod adapters;
mod cmd;
mod project;

#[macro_export]
macro_rules! throw {
  ($($arg:tt)*) => {{
    return Err(format!($($arg)*))
  }};
}

#[command]
fn error_popup(msg: String, win: Window) {
  println!("Error popup: {}", msg);
  thread::spawn(move || {
    dialog::message(Some(&win), "Error", msg);
  });
}

fn handle_open_files(files: &[String], app: &AppHandle) -> Vec<String> {
  let files = files
    .iter()
    .map(|f| {
      percent_encoding::percent_decode(f.as_bytes())
        .decode_utf8_lossy()
        .into_owned()
    })
    .collect::<Vec<String>>();
  app
    .emit_all("open-file", &files)
    .expect("open file event failed");
  files
}

#[command]
fn opened_info(state: State<OpenedInfoState>) -> OpenedInfo {
  let opened_info = state.0.lock().expect("no opened info state");
  (*opened_info).clone()
}

#[derive(Serialize, Deserialize, Clone, TypeDef)]
pub struct OpenedInfo {
  path: Option<String>,
}
pub struct OpenedInfoState(pub Mutex<OpenedInfo>);

fn main() {
  #[cfg(debug_assertions)]
  typegen();

  let ctx = tauri::generate_context!();

  tauri::Builder::default()
    .setup(|app| {
      let _ = WindowBuilder::new(app, "main", WindowUrl::default())
        .title("Pivoteer")
        .resizable(true)
        .decorations(true)
        .always_on_top(false)
        .inner_size(750.0, 600.0)
        .min_inner_size(730.0, 350.0)
        .fullscreen(false)
        .build()
        .expect("Unable to create window");
      let mut _opened_info = OpenedInfo { path: None };

      #[cfg(any(windows, target_os = "linux"))]
      {
        // Windows and Linux
        let argv = env::args().collect::<Vec<_>>();
        if argv.len() > 1 {
          // NOTICE: `argv` may include URL protocol (`your-app-protocol://`) or arguments (`--`) if app supports them.
          let files = handle_open_files(&argv[1..], app.handle());
          opened_info.path = files.remove(0);
        }
      }

      app.manage(OpenedInfoState(Mutex::new(OpenedInfo { path: None })));
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      error_popup,
      opened_info,
      cmd::generate,
      cmd::open,
      cmd::save,
      cmd::set_edited,
    ])
    .menu(Menu::with_items([
      #[cfg(target_os = "macos")]
      MenuEntry::Submenu(Submenu::new(
        &ctx.package_info().name,
        Menu::with_items([
          MenuItem::About(ctx.package_info().name.clone(), AboutMetadata::default()).into(),
          MenuItem::Separator.into(),
          MenuItem::Services.into(),
          MenuItem::Separator.into(),
          MenuItem::Hide.into(),
          MenuItem::HideOthers.into(),
          MenuItem::ShowAll.into(),
          MenuItem::Separator.into(),
          MenuItem::Quit.into(),
        ]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "File",
        Menu::with_items([
          CustomMenuItem::new("New", "New")
            .accelerator("cmdOrControl+N")
            .into(),
          CustomMenuItem::new("Open...", "Open...")
            .accelerator("cmdOrControl+O")
            .into(),
          MenuItem::Separator.into(),
          CustomMenuItem::new("Close", "Close")
            .accelerator("cmdOrControl+W")
            .into(),
          CustomMenuItem::new("Save", "Save")
            .accelerator("cmdOrControl+S")
            .into(),
        ]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "Edit",
        Menu::with_items([
          MenuItem::Undo.into(),
          MenuItem::Redo.into(),
          MenuItem::Separator.into(),
          MenuItem::Cut.into(),
          MenuItem::Copy.into(),
          MenuItem::Paste.into(),
          #[cfg(not(target_os = "macos"))]
          MenuItem::Separator.into(),
          MenuItem::SelectAll.into(),
        ]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "View",
        Menu::with_items([MenuItem::EnterFullScreen.into()]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "Window",
        Menu::with_items([MenuItem::Minimize.into(), MenuItem::Zoom.into()]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "Help",
        Menu::with_items([CustomMenuItem::new("Learn More", "Learn More").into()]),
      )),
    ]))
    .on_menu_event(|event| {
      let event_name = event.menu_item_id();
      event.window().emit("menu", event_name).unwrap();
      match event_name {
        "Learn More" => {
          let url = "https://youtu.be/dQw4w9WgXcQ".to_string();
          shell::open(&event.window().shell_scope(), url, None).unwrap();
        }
        _ => {}
      }
    })
    .build(ctx)
    .expect("error while running tauri app")
    .run(|app, event| {
      #[cfg(target_os = "macos")]
      if let tauri::RunEvent::OpenURLs(urls) = event {
        // filter out non-file:// urls, you may need to handle them by another method
        let file_paths: Vec<_> = urls
          .iter()
          .filter_map(|url| {
            if url.scheme() == "file" {
              Some(url.path().into())
            } else {
              None
            }
          })
          .collect();

        let mut files = handle_open_files(&file_paths, &app);
        let opened_info_state = app.state::<OpenedInfoState>();
        let mut opened_info = opened_info_state.0.lock().expect("no opened info state");
        opened_info.path = Some(files.remove(0));
      }
    });
}

#[cfg(debug_assertions)]
pub fn typegen() {
  use typescript_type_def::{write_definition_file, DefinitionFileOptions};

  use crate::project::Project;
  let mut file = std::fs::File::create("../bindings.ts").unwrap();
  let options = DefinitionFileOptions {
    root_namespace: None,
    ..Default::default()
  };
  write_definition_file::<_, Project>(&mut file, options).unwrap();
  write_definition_file::<_, OpenedInfo>(&mut file, options).unwrap();
  println!("Generated TS types");
}
