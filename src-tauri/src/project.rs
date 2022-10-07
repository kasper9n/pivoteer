use crate::adapters::{adapter, Adapter};
use serde::Deserialize;
use typescript_type_def::{write_definition_file, DefinitionFileOptions, TypeDef};

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
#[derive(Deserialize, Debug, TypeDef)]
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
  pub source_type: SourceType,
}

#[derive(Deserialize, Debug, TypeDef)]
pub enum SourceType {
  Landr,
}

impl SourceType {
  pub fn adapter(&self) -> impl Adapter {
    adapter(self)
  }
}

#[cfg(debug_assertions)]
pub fn typegen() {
  let mut file = std::fs::File::create("./bindings.ts").unwrap();
  let options = DefinitionFileOptions {
    root_namespace: None,
    ..Default::default()
  };
  write_definition_file::<_, Project>(&mut file, options).unwrap();
  println!("Generated TS types");
}
