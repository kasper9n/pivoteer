use crate::cmd::get_cell;
use crate::project::{ColumnConfig, ColumnType, SourceConfig, SourceType};
use crate::throw;
use csv::StringRecord;
use std::fs::File;
use std::io::BufReader;

pub struct CsvRow {
  pub record: StringRecord,
}
impl CsvRow {
  fn find_by_name(&self, name: &str) -> Result<usize, String> {
    let index = self.record.iter().position(|s| s == name);
    index.ok_or(format!("No column named {name}"))
  }
  fn get(&self, index: usize) -> Result<String, String> {
    Ok(get_cell(&self.record, index)?.to_owned())
  }
}

type CsvIter = csv::StringRecordsIntoIter<BufReader<File>>;
pub struct CsvHeader<'a> {
  records: &'a mut CsvIter,
  row: Option<CsvRow>,
}
impl CsvHeader<'_> {
  fn skip_rows(&mut self, rows: usize) -> Result<(), String> {
    println!("Skipping {rows} pre-header rows");
    for _ in 0..rows {
      let row = self.records.next();
      if row.is_none() {
        throw!("Non-existant {rows} pre-header rows");
      }
    }
    Ok(())
  }
  fn get(&mut self) -> Result<&CsvRow, String> {
    match self.row {
      Some(ref row) => return Ok(row),
      None => {}
    };
    self.row = match self.records.next() {
      Some(Ok(record)) => Some(CsvRow { record }),
      Some(Err(e)) => throw!("Error reading headers: {}", e.to_string()),
      None => throw!("No header row"),
    };
    match self.row {
      Some(ref row) => return Ok(row),
      None => panic!(),
    };
  }
  fn find_by_name(&mut self, name: &str) -> Result<usize, String> {
    self.get()?.find_by_name(name)
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

pub fn adapter<'a>(kind: SourceType, records: &'a mut CsvIter) -> Result<Adapter, String> {
  let mut header = CsvHeader { records, row: None };
  let adapter = match kind {
    SourceType::Bandcamp => wrap(Bandcamp::new(&mut header)?),
    SourceType::Landr => wrap(Landr::new(&mut header)?),
    SourceType::Pretzel => wrap(Pretzel::new(&mut header)?),
    SourceType::RepostBySoundCloud => wrap(RepostBySoundCloud::new(&mut header)?),
    SourceType::Stem => wrap(Stem::new(&mut header)?),
    SourceType::Symphonic => wrap(Symphonic::new(&mut header)?),
    SourceType::Custom(config) => wrap(CustomSource::new(&mut header, config)?),
  };
  Ok(adapter)
}

struct Bandcamp {
  isrc: usize,
  upc: usize,
  net_amount: usize,
}
impl Bandcamp {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      isrc: header.find_by_name("isrc")?,
      upc: header.find_by_name("upc")?,
      net_amount: header.find_by_name("net amount")?,
    })
  }
}
impl AdapterT for Bandcamp {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.net_amount),
    }
  }
}

struct Landr {
  isrc: usize,
  upc: usize,
  net_earnings: usize,
}
impl Landr {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      isrc: header.find_by_name("ISRC")?,
      upc: header.find_by_name("UPC")?,
      net_earnings: header.find_by_name("Net earnings (USD)")?,
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
      isrc: header.find_by_name("isrc")?,
      total_revenue: header.find_by_name("total_revenue")?,
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
      isrc: header.find_by_name("ISRC")?,
      upc: header.find_by_name("UPC")?,
      revenue: header.find_by_name("Revenue (USD)")?,
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
      isrc: header.find_by_name("isrc")?,
      upc: header.find_by_name("upc")?,
      net_royalties: header.find_by_name("net_royalties")?,
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
      isrc: header.find_by_name("ISRC Code")?,
      upc: header.find_by_name("UPC Code")?,
      net_royalties: header.find_by_name("Royalty ($US)")?,
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
    let header_row = header.get()?;
    Ok(Self {
      isrc: Column::from_config(config.isrc, header_row)?,
      upc: Column::from_config(config.upc, header_row)?,
      revenue: Column::from_config(config.revenue, header_row)?,
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
        let index = header.record.iter().position(|s| s == name);
        let index = index.ok_or(format!("No column named {name}"))?;
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
