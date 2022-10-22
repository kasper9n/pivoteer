use serde::{Deserialize, Serialize};
use strum_macros::Display;
use typescript_type_def::TypeDef;

use crate::adapters::CsvHeader;
use crate::throw;

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
  pub filters: Vec<FilterConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, TypeDef)]
pub enum FilterOperator {
  Is,
  IsNot,
}

#[derive(Serialize, Deserialize, Clone, Debug, TypeDef)]
pub struct FilterConfig {
  pub column: ColumnLocation,
  pub operator: FilterOperator,
  pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, TypeDef)]
pub enum ColumnLocation {
  Name(String),
  Index(usize),
  NameAtIndex(String, usize),
}
impl ColumnLocation {
  pub fn index_from_header(&self, header: &CsvHeader) -> Result<usize, String> {
    match self {
      ColumnLocation::Name(name) => header.position(name),
      ColumnLocation::Index(index) => {
        header.row.get(*index)?;
        Ok(*index)
      }
      ColumnLocation::NameAtIndex(name, index) => {
        let actual_name = header
          .row
          .record
          .get(*index)
          .ok_or(format!("No column number {}", index + 1))?;
        if actual_name != name {
          throw!("No column number {} named {}", index + 1, name);
        }
        Ok(*index)
      }
    }
  }
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
