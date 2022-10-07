use serde::Deserialize;
use strum_macros::Display;
use typescript_type_def::TypeDef;

#[derive(Deserialize, Debug, TypeDef)]
pub struct Project {
  pub columns: Vec<Column>,
  pub sources: Vec<Source>,
}

#[derive(Deserialize, Debug, TypeDef)]
pub struct Column {
  pub kind: ColumnType,
  pub enabled: bool,
  pub name: String,
  pub action: Action,
}
#[derive(Deserialize, Clone, Copy, Debug, TypeDef)]
pub enum ColumnType {
  Isrc,
  Upc,
  NetEarnings,
}

#[derive(Deserialize, Clone, Copy, Debug, TypeDef)]
pub enum Action {
  Unique,
  Sum,
}

#[derive(Deserialize, Debug, TypeDef)]
pub struct Source {
  pub name: String,
  pub files: Vec<String>,
  pub kind: SourceType,
}

#[derive(Deserialize, Clone, Copy, Display, Debug, TypeDef)]
pub enum SourceType {
  Landr,
  Pretzel,
  RepostBySoundCloud,
}

#[cfg(debug_assertions)]
pub fn typegen() {
  use typescript_type_def::{write_definition_file, DefinitionFileOptions};
  let mut file = std::fs::File::create("../bindings.ts").unwrap();
  let options = DefinitionFileOptions {
    root_namespace: None,
    ..Default::default()
  };
  write_definition_file::<_, Project>(&mut file, options).unwrap();
  println!("Generated TS types");
}
