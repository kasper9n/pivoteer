use crate::adapters::{adapter, Adapter, CsvRow};
use crate::project::{Action, Column, ColumnType, Project, Source, SourceType};
use crate::throw;
use bigdecimal::BigDecimal;
use csv::{self, Reader};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;
use tauri::command;

pub fn get_cell<'a>(record: &'a csv::StringRecord, index: usize) -> Result<&'a str, String> {
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

fn read_csv(file_path: &Path) -> Result<Reader<BufReader<File>>, String> {
  let buf_reader = match File::open(file_path.clone()) {
    Ok(file) => BufReader::new(file),
    Err(e) => throw!("Error opening csv: {}", e.to_string()),
  };
  let ext = file_path.extension().unwrap_or_default().to_string_lossy();
  let delimiter = match ext.as_ref() {
    "tsv" => b'\t',
    "csv" => b',',
    ext => throw!("Unsupported file extension {}", ext),
  };
  let rdr = csv::ReaderBuilder::new()
    .delimiter(delimiter)
    .has_headers(false)
    .from_reader(buf_reader);
  Ok(rdr)
}

#[derive(Debug)]
pub struct FoundColumn {
  pub action: Action,
  pub kind: ColumnType,
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
    adapter: &Adapter,
    record: csv::StringRecord,
    key_cols: &Vec<FoundColumn>,
    value_cols: &Vec<FoundColumn>,
  ) -> Result<(), String> {
    let row = CsvRow { record };
    let mut keys: Vec<String> = Vec::new();
    for key_col in key_cols {
      let cell = adapter.get(&row, &key_col.kind)?;
      keys.push(cell);
    }

    let values = self.map.entry(keys).or_insert(Vec::new());
    for (i, col) in value_cols.iter().enumerate() {
      let cell = adapter.get(&row, &col.kind)?;
      let value = str_to_dec(&cell)?;
      match col.action {
        Action::Unique => {}
        Action::Sum => {
          match values.get_mut(i) {
            Some(v) => *v += value,
            None => values.push(value),
          };
        }
      }
    }
    Ok(())
  }

  pub fn add_csv(&mut self, file_path: &String, kind: SourceType) -> Result<(), String> {
    let file_path = Path::new(&file_path);
    let mut csv = read_csv(file_path)?.into_records();

    let adapter = adapter(kind, &mut csv)?;

    let mut key_cols = Vec::new();
    let mut value_cols = Vec::new();
    for column in &self.columns {
      if !column.enabled {
        continue;
      }
      let found_col = FoundColumn {
        action: column.action,
        kind: column.kind,
      };
      match column.action {
        Action::Unique => key_cols.push(found_col),
        _ => value_cols.push(found_col),
      };
    }

    for record_result in csv {
      let record: csv::StringRecord = match record_result {
        Ok(record) => record,
        Err(e) => throw!("Error reading record: {}", e.to_string()),
      };
      self.add_csv_record(&adapter, record, &key_cols, &value_cols)?;
    }
    Ok(())
  }

  pub fn add_source(&mut self, source: Source) -> Result<(), String> {
    for file in source.files {
      let filename = Path::new(&file)
        .file_name()
        .unwrap_or(OsStr::new(""))
        .to_string_lossy();
      match self.add_csv(&file, source.kind.clone()) {
        Ok(_) => println!("Scanned {}", filename),
        Err(e) => throw!("{} - Failed scanning {filename}: {e}", source.kind),
      };
    }
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
  project.sources.into_iter().try_for_each(|source| {
    return aggregator.add_source(source);
  })?;
  let output = aggregator.output()?;

  let dur = Instant::now().duration_since(start).as_nanos() as f32;
  println!("\u{23f1}  {:.3}ms", dur / 1000.0 / 1000.0);

  return Ok(output);
}
