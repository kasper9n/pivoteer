use bigdecimal::{BigDecimal, Zero};
use csv;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use tauri::command;

fn get_cell<'a>(record: &'a csv::StringRecord, index: usize) -> Result<&'a str, String> {
  return match record.get(index) {
    Some(cell) => Ok(cell),
    None => Err(format!("No value found for cell {}", index)),
  };
}

fn str_to_dec<'a>(input: &'a str) -> Result<BigDecimal, String> {
  return match BigDecimal::from_str(&input) {
    Ok(num) => Ok(num),
    Err(e) => Err(format!("Invalid number: {}", e.to_string())),
  };
}

struct Aggregator {
  map: HashMap<Vec<String>, BigDecimal>,
}

impl Aggregator {
  pub fn new() -> Self {
    return Aggregator {
      map: HashMap::new(),
    };
  }

  pub fn add_csv(&mut self, file_path: String) -> Result<(), String> {
    let mut rdr = match csv::Reader::from_path(file_path) {
      Ok(reader) => reader,
      Err(e) => return Err("Error opening csv: ".to_string() + &e.to_string()),
    };
    let mut i = 0;
    for result in rdr.records() {
      let record: csv::StringRecord = match result {
        Ok(record) => record,
        Err(e) => return Err("Error reading record: ".to_string() + &e.to_string()),
      };
      // let start_date = get_cell(&record, 0)?;
      // let upc = get_cell(&record, 6)?;
      // let isrc = get_cell(&record, 8)?;
      // let value = str_to_dec(&get_cell(&record, 9)?)?; // quantity of sales or streams
      let value = str_to_dec(&get_cell(&record, 11)?)?; // net earnings
      let mut index = Vec::new();
      index.push(get_cell(&record, 0)?.into()); // start date
      index.push(get_cell(&record, 2)?.into()); // Store
      index.push(get_cell(&record, 3)?.into()); // Store service
      index.push(get_cell(&record, 8)?.into()); // isrc
      index.push(get_cell(&record, 6)?.into()); // upc
                                                // let store = get_cell(&record, 2)?;
      let isrc = get_cell(&record, 8)?;
      if isrc == "QM24S1832184" {
        *self.map.entry(index).or_insert(BigDecimal::zero()) += value;
      }
      if i % 10000 == 0 {
        println!("{}", i);
        // break;
      }
      i += 1;
    }
    return Ok(());
  }

  pub fn output(&mut self) -> Result<String, String> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    for (indexes, value) in &self.map {
      let mut record = Vec::new();
      for index in indexes {
        record.push(index);
      }
      let value = value.to_string();
      record.push(&value);
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

// // Non-working code for emitting events from
// #[command(with_window)]
// pub fn import_csv<M: tauri::Params>(
//   window: tauri::Window<M>,
//   event: String,
//   payload: Option<String>,
// ) -> Result<(), String> {
//   window.emit_others("output".to_string(), Some("test".to_owned()));
//   // window
//   //   .emit(&"output".into(), Some("test".to_owned()))
//   //   .expect("failed to emit");

//   return Ok(());
// }

#[command]
pub fn import_csv(file_path: String) -> Result<String, String> {
  let start = Instant::now();

  let mut agg = Aggregator::new();
  agg.add_csv(file_path)?;
  let output = agg.output()?;
  println!("{}", output);

  let dur = Instant::now().duration_since(start).as_nanos() as f32;
  println!("\u{23f1}  {:.3}ms", dur / 1000.0 / 1000.0);

  return Ok(output);
}
