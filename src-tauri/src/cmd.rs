use crate::adapters::Adapter;
use crate::project::{Action, Column, Project};
use crate::throw;
use bigdecimal::BigDecimal;
use csv::{self, Reader, StringRecord};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;
use tauri::command;

fn get_cell<'a>(record: &'a csv::StringRecord, index: usize) -> Result<&'a str, String> {
  return match record.get(index) {
    Some(cell) => Ok(cell),
    None => Err(format!(
      "No value found for cell at column number {}",
      index + 1
    )),
  };
}

fn str_to_dec<'a>(input: &'a str) -> Result<BigDecimal, String> {
  return match BigDecimal::from_str(&input) {
    Ok(num) => Ok(num),
    Err(e) => Err(format!("Invalid number: {}", e.to_string())),
  };
}

fn read_csv(file_path: &Path, header_row_index: usize) -> Result<Reader<BufReader<File>>, String> {
  let mut buf_reader = match File::open(file_path.clone()) {
    Ok(file) => BufReader::new(file),
    Err(e) => throw!("Error opening csv: {}", e.to_string()),
  };
  for i in 0..header_row_index {
    println!("Skipping row index {i}");
    let mut s = "".to_string();
    match buf_reader.read_line(&mut s) {
      Ok(_) => {}
      Err(e) => throw!("Error skipping pre-header rows: {}", e.to_string()),
    }
  }
  let ext = file_path.extension().unwrap_or_default().to_string_lossy();
  let delimiter = match ext.as_ref() {
    "tsv" => b'\t',
    "csv" => b',',
    ext => throw!("Unsupported file extension {}", ext),
  };
  let rdr = csv::ReaderBuilder::new()
    .delimiter(delimiter)
    .from_reader(buf_reader);
  Ok(rdr)
}

#[allow(dead_code)]
pub enum ColumnLocation<'a> {
  Name(&'a str),
  Index(usize),
  NameAtIndex(&'a str, usize),
}
impl ColumnLocation<'_> {
  pub fn find(&self, headers: &StringRecord) -> Result<usize, String> {
    match self {
      ColumnLocation::Name(name) => {
        let index = headers.iter().position(|s| s == *name);
        index.ok_or(format!("No column named {name}"))
      }
      ColumnLocation::Index(index) => {
        headers
          .get(*index)
          .ok_or(format!("No column number {}", index + 1))?;
        Ok(*index)
      }
      ColumnLocation::NameAtIndex(name, index) => {
        let actual_name = headers
          .get(*index)
          .ok_or(format!("No column number {}", index + 1))?;
        if actual_name != *name {
          throw!("No column number {} named {}", index + 1, name);
        }
        Ok(*index)
      }
    }
  }
}

#[derive(Debug)]
pub struct FoundColumn {
  pub action: Action,
  pub index: usize,
}

struct Aggregator {
  map: HashMap<Vec<String>, Vec<BigDecimal>>,
  columns: Vec<Column>,
}

impl Aggregator {
  pub fn new(columns: Vec<Column>) -> Self {
    return Aggregator {
      map: HashMap::new(),
      columns,
    };
  }

  pub fn add_csv_record(
    &mut self,
    record: &csv::StringRecord,
    key_cols: &Vec<FoundColumn>,
    value_cols: &Vec<FoundColumn>,
  ) -> Result<(), String> {
    let mut keys: Vec<String> = Vec::new();
    for key_col in key_cols {
      let cell = get_cell(&record, key_col.index)?.to_owned();
      keys.push(cell);
    }

    let values = self.map.entry(keys).or_insert(Vec::new());
    for (i, col) in value_cols.iter().enumerate() {
      match col.action {
        Action::Unique => {}
        Action::Sum => {
          let cell = get_cell(&record, col.index)?.into();
          let value = str_to_dec(cell)?;
          match values.get_mut(i) {
            Some(v) => {
              *v += value;
            }
            None => values.push(value),
          };
        }
      }
    }
    Ok(())
  }

  pub fn add_csv(&mut self, file_path: String, adapter: &impl Adapter) -> Result<(), String> {
    let file_path = Path::new(&file_path);
    let mut rdr = read_csv(file_path, adapter.header_row_index())?;
    let headers = match rdr.headers() {
      Ok(headers) => headers,
      Err(e) => throw!("Error reading headers: {}", e.to_string()),
    };

    let mut key_cols = Vec::new();
    let mut value_cols = Vec::new();
    for column in &self.columns {
      if !column.enabled {
        continue;
      }
      let found_col = FoundColumn {
        action: column.action,
        index: adapter.column_location(&column.kind).find(headers)?,
      };
      match column.action {
        Action::Unique => key_cols.push(found_col),
        _ => value_cols.push(found_col),
      };
    }

    for record_result in rdr.into_records() {
      let record: csv::StringRecord = match record_result {
        Ok(record) => record,
        Err(e) => throw!("Error reading record: {}", e.to_string()),
      };
      self.add_csv_record(&record, &key_cols, &value_cols)?;
    }

    let filename = file_path.file_name().unwrap_or(OsStr::new(""));
    println!("Scanned {}", filename.to_string_lossy());
    Ok(())
  }

  pub fn output(&mut self) -> Result<String, String> {
    let mut wtr = csv::WriterBuilder::new()
      .quote_style(csv::QuoteStyle::Always)
      .from_writer(Vec::new());

    let mut header = Vec::new();
    for column in &self.columns {
      header.push(column.name.clone());
    }
    match wtr.write_record(header) {
      Ok(_) => {}
      Err(e) => return Err(format!("Error writing header: {}", e)),
    };

    for (indexes, values) in &self.map {
      let mut indexes_iter = indexes.iter();
      let mut values_iter = values.iter();
      let mut record = Vec::new();
      for column in &self.columns {
        match column.action {
          Action::Unique => match indexes_iter.next() {
            Some(index_value) => record.push(index_value.clone()),
            None => throw!("No value found for column to output"),
          },
          Action::Sum => match values_iter.next() {
            Some(value) => record.push(value.to_string()),
            None => throw!("No value found for column to output"),
          },
        }
      }
      match wtr.write_record(record) {
        Ok(_) => {}
        Err(e) => return Err(format!("Error writing record: {}", e)),
      }
    }
    let inner_wtr = match wtr.into_inner() {
      Ok(inner_wtr) => inner_wtr,
      Err(e) => return Err(format!("Error reading record: {}", e.to_string())),
    };
    let output = match String::from_utf8(inner_wtr) {
      Ok(output) => output,
      Err(e) => return Err(format!("Error reading record: {}", e.to_string())),
    };
    Ok(output)
  }
}

#[command]
pub async fn generate(project: Project) -> Result<String, String> {
  let start = Instant::now();

  let mut aggregator = Aggregator::new(project.columns);
  for source in project.sources {
    let adapter = source.source_type.adapter();
    for file in source.files {
      aggregator.add_csv(file, &adapter)?;
    }
  }
  let output = aggregator.output()?;

  let dur = Instant::now().duration_since(start).as_nanos() as f32;
  println!("\u{23f1}  {:.3}ms", dur / 1000.0 / 1000.0);

  return Ok(output);
}
