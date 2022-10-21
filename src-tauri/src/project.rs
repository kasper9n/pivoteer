use serde::{Deserialize, Serialize};
use strum_macros::Display;
use typescript_type_def::TypeDef;

#[derive(Serialize, Deserialize, Debug, TypeDef)]
pub struct Project {
  pub columns: Vec<Column>,
  pub sources: Vec<Source>,
}

#[derive(Serialize, Deserialize, Debug, TypeDef)]
pub struct Column {
  pub kind: ColumnType,
  pub enabled: bool,
  pub name: String,
  pub action: Action,
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, TypeDef)]
pub enum ColumnType {
  Isrc,
  Upc,
  NetEarnings,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, TypeDef)]
pub enum Action {
  Unique,
  Sum,
}

#[derive(Serialize, Deserialize, Debug, TypeDef)]
pub struct Source {
  pub name: String,
  pub files: Vec<String>,
  pub kind: SourceType,
}

#[derive(Serialize, Deserialize, Clone, Display, Debug, TypeDef)]
#[serde(tag = "id", content = "content")]
pub enum SourceType {
  Bandcamp,
  Landr,
  Pretzel,
  RepostBySoundCloud,
  Stem,
  Symphonic,
  Custom(SourceConfig),
}

#[derive(Serialize, Deserialize, Clone, Debug, TypeDef)]
pub struct SourceConfig {
  pub header_row_index: usize,
  pub isrc: Option<ColumnConfig>,
  pub upc: Option<ColumnConfig>,
  pub revenue: Option<ColumnConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, TypeDef)]
pub enum ColumnConfig {
  Name(String),
  Index(usize),
  NameAtIndex(String, usize),
  CustomValue(String),
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
