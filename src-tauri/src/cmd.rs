use crate::throw;
use bigdecimal::{BigDecimal, Zero};
use csv;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;
use tauri::command;

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Source {
  pub name: String,
  pub files: Vec<String>,
  pub headerRowIndex: usize,
  pub columns: Vec<Column>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Column {
  pub action: Action,
  pub idType: IdType,
  pub id: String,
}

#[derive(Deserialize, Debug)]
pub enum IdType {
  Name,
  Number,
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub enum Action {
  Unique,
  Sum,
}

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

  pub fn add_csv(
    &mut self,
    file_path: String,
    header_row_index: usize,
  ) -> Result<BigDecimal, String> {
    let file_path = Path::new(&file_path);
    let filename = file_path.file_name().unwrap_or(OsStr::new(""));
    let mut buf_reader = match File::open(file_path.clone()) {
      Ok(file) => BufReader::new(file),
      Err(e) => throw!("Error opening csv: {}", e.to_string()),
    };
    for _ in 0..header_row_index {
      println!("Skipping...");
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
    let mut rdr = csv::ReaderBuilder::new()
      .delimiter(delimiter)
      .from_reader(buf_reader);
    let headers = match rdr.headers() {
      Ok(headers) => headers,
      Err(e) => throw!("Error reading headers: {}", e.to_string()),
    };

    #[derive(Debug)]
    pub struct NumberedColumn {
      pub action: Action,
      pub index: usize,
    }
    let mut indexed_columns: Vec<NumberedColumn> = Vec::new();
    let mut value_columns: Vec<NumberedColumn> = Vec::new();
    for column in &self.columns {
      let numbered_col = match column.idType {
        IdType::Number => {
          let index = match column.id.parse() {
            Ok(0) | Err(_) => throw!("Invalid column number: {}", column.id),
            Ok(num) => num - 1,
          };
          NumberedColumn {
            action: column.action,
            index,
          }
        }
        IdType::Name => {
          let index = match headers.iter().position(|s| s == &column.id) {
            Some(index) => index,
            None => throw!("No column found named {}", column.id),
          };
          NumberedColumn {
            action: column.action,
            index,
          }
        }
      };
      match column.action {
        Action::Unique => indexed_columns.push(numbered_col),
        Action::Sum => value_columns.push(numbered_col),
      };
    }

    let mut _i = 0;
    let mut csvtotal = BigDecimal::zero();
    for result in rdr.records() {
      _i += 1;
      let record: csv::StringRecord = match result {
        Ok(record) => record,
        Err(e) => throw!("Error reading record: {}", e.to_string()),
      };

      let mut indexes: Vec<String> = Vec::new();
      for col in &indexed_columns {
        match col.action {
          Action::Unique => {
            let cell = get_cell(&record, col.index)?.into();
            indexes.push(cell);
          }
          _ => {}
        }
      }
      let values = self.map.entry(indexes).or_insert(Vec::new());
      for (vi, col) in value_columns.iter().enumerate() {
        match col.action {
          Action::Unique => {}
          Action::Sum => {
            let cell = get_cell(&record, col.index)?.into();
            let value = str_to_dec(cell)?;
            match values.get_mut(vi) {
              Some(v) => {
                csvtotal += value.clone();
                *v += value;
              }
              None => values.push(value),
            };
          }
        }
      }
      // if i % 10000 == 0 {
      //   progress bar
      // }
    }
    println!(
      "Sum of values: {} in {}",
      csvtotal,
      filename.to_string_lossy()
    );
    return Ok(csvtotal);
  }

  pub fn output(&mut self) -> Result<String, String> {
    let mut wtr = csv::WriterBuilder::new()
      .quote_style(csv::QuoteStyle::Always)
      .from_writer(Vec::new());
    {
      let mut header = Vec::new();
      for column in &self.columns {
        header.push(column.id.clone());
      }
      match wtr.write_record(header) {
        Ok(_) => {}
        Err(e) => return Err(format!("Error writing header: {}", e)),
      };
    }
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
pub async fn generate(source: Source) -> Result<String, String> {
  let start = Instant::now();

  let mut agg = Aggregator::new(source.columns);
  let mut sum_all = BigDecimal::zero();
  for file in source.files {
    let sum = agg.add_csv(file, source.headerRowIndex)?;
    sum_all += sum;
  }
  let output = agg.output()?;
  println!("Sum of values, all files: {}", sum_all);

  let dur = Instant::now().duration_since(start).as_nanos() as f32;
  println!("\u{23f1}  {:.3}ms", dur / 1000.0 / 1000.0);

  return Ok(output);
}
