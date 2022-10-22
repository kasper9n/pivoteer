use crate::cmd::get_cell;
use crate::project::{ColumnConfig, ColumnType, SourceConfig, SourceType};
use crate::throw;
use csv::{ReaderBuilder, StringRecord};
use std::fs::File;
use std::io::BufReader;

pub struct CsvRow {
  pub record: StringRecord,
}
impl CsvRow {
  pub fn position(&self, name: &str) -> Result<usize, String> {
    let index = self.record.iter().position(|s| s == name);
    index.ok_or(format!("No column named {name}"))
  }
  pub fn get(&self, index: usize) -> Result<String, String> {
    Ok(get_cell(&self.record, index)?.to_owned())
  }
}

type CsvIter = csv::StringRecordsIntoIter<BufReader<File>>;
pub struct CsvHeader<'a> {
  records: &'a mut CsvIter,
  pub row: CsvRow,
}
impl<'a> CsvHeader<'a> {
  fn skip_rows(&mut self, rows: usize) -> Result<(), String> {
    println!("Skipping {rows} pre-header rows");
    for _ in 0..rows {
      let record = match self.records.next() {
        Some(Ok(record)) => record,
        Some(Err(e)) => throw!("Error reading a pre-header row: {e}"),
        None => throw!("No header row"),
      };
      self.row = CsvRow { record };
    }
    Ok(())
  }
  pub fn from_records(records: &'a mut CsvIter) -> Result<Self, String> {
    let row = match records.next() {
      Some(Ok(record)) => CsvRow { record },
      Some(Err(e)) => throw!("Error reading headers: {}", e.to_string()),
      None => throw!("No header row"),
    };
    Ok(Self { records, row })
  }
  pub fn position(&self, name: &str) -> Result<usize, String> {
    self.row.position(name)
  }
}

pub type Adapter = Box<dyn AdapterT>;
pub trait AdapterT {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String>;
}

fn wrap(v: impl AdapterT + 'static) -> Adapter {
  let b = Box::new(v);
  b
}

pub fn adapter_csv_settings(kind: &SourceType, reader_builder: &mut ReaderBuilder) {
  match kind {
    SourceType::RepostBySoundCloud => {
      reader_builder.flexible(true);
    }
    _ => {}
  }
}
pub fn adapter<'a>(kind: SourceType, header: &mut CsvHeader) -> Result<Adapter, String> {
  let adapter = match kind {
    SourceType::Landr => wrap(Landr::new(header)?),
    SourceType::Pretzel => wrap(Pretzel::new(header)?),
    SourceType::RepostBySoundCloud => wrap(RepostBySoundCloud::new(header)?),
    SourceType::Stem => wrap(Stem::new(header)?),
    SourceType::Symphonic => wrap(Symphonic::new(header)?),
    SourceType::Custom(config) => wrap(CustomSource::new(header, config)?),
  };
  Ok(adapter)
}

struct Landr {
  isrc: usize,
  upc: usize,
  net_earnings: usize,
}
impl Landr {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      isrc: header.position("ISRC")?,
      upc: header.position("UPC")?,
      net_earnings: header.position("Net earnings (USD)")?,
    })
  }
}
impl AdapterT for Landr {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.net_earnings),
    }
  }
}

struct Pretzel {
  isrc: usize,
  total_revenue: usize,
}
impl Pretzel {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      isrc: header.position("isrc")?,
      total_revenue: header.position("total_revenue")?,
    })
  }
}
impl AdapterT for Pretzel {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => Ok("".to_owned()),
      ColumnType::NetEarnings => row.get(self.total_revenue),
    }
  }
}

struct RepostBySoundCloud {
  isrc: usize,
  upc: usize,
  revenue: usize,
}
impl RepostBySoundCloud {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    header.skip_rows(2)?;
    Ok(Self {
      isrc: header.position("ISRC")?,
      upc: header.position("UPC")?,
      revenue: header.position("Revenue (USD)")?,
    })
  }
}
impl AdapterT for RepostBySoundCloud {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.revenue),
    }
  }
}

struct Stem {
  isrc: usize,
  upc: usize,
  net_royalties: usize,
}
impl Stem {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      isrc: header.position("isrc")?,
      upc: header.position("upc")?,
      net_royalties: header.position("net_royalties")?,
    })
  }
}
impl AdapterT for Stem {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.net_royalties),
    }
  }
}

struct Symphonic {
  isrc: usize,
  upc: usize,
  net_royalties: usize,
}
impl Symphonic {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      isrc: header.position("ISRC Code")?,
      upc: header.position("UPC Code")?,
      net_royalties: header.position("Royalty ($US)")?,
    })
  }
}
impl AdapterT for Symphonic {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.net_royalties),
    }
  }
}

pub struct CustomSource {
  pub isrc: Option<Column>,
  pub upc: Option<Column>,
  pub revenue: Option<Column>,
}
impl CustomSource {
  fn new(header: &mut CsvHeader, config: SourceConfig) -> Result<Self, String> {
    header.skip_rows(config.header_row_index)?;
    Ok(Self {
      isrc: Column::from_config(config.isrc, &header.row)?,
      upc: Column::from_config(config.upc, &header.row)?,
      revenue: Column::from_config(config.revenue, &header.row)?,
    })
  }
}
impl AdapterT for CustomSource {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    let column_field = match &kind {
      ColumnType::Isrc => &self.isrc,
      ColumnType::Upc => &self.upc,
      ColumnType::NetEarnings => &self.revenue,
    };
    let column = match column_field {
      Some(column) => column,
      None => throw!("Unsupported column {:?}", kind),
    };
    match column {
      Column::Index(index) => row.get(*index),
      Column::CustomValue(custom_value) => Ok(custom_value.clone()),
    }
  }
}

pub enum Column {
  Index(usize),
  CustomValue(String),
}
impl Column {
  fn from_config(config: Option<ColumnConfig>, header: &CsvRow) -> Result<Option<Self>, String> {
    let config = match config {
      Some(config) => config,
      None => return Ok(None),
    };
    let column = match config {
      ColumnConfig::Name(name) => {
        let index = header.position(&name)?;
        Column::Index(index)
      }
      ColumnConfig::Index(index) => {
        header.get(index)?;
        Column::Index(index)
      }
      ColumnConfig::NameAtIndex(name, index) => {
        let actual_name = header
          .record
          .get(index)
          .ok_or(format!("No column number {}", index + 1))?;
        if actual_name != name {
          throw!("No column number {} named {}", index + 1, name);
        }
        Column::Index(index)
      }
      ColumnConfig::CustomValue(value) => Column::CustomValue(value),
    };
    Ok(Some(column))
  }
}
