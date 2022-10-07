use crate::cmd::get_cell;
use crate::project::{ColumnType, SourceType};
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

pub fn adapter<'a>(kind: &SourceType, records: &'a mut CsvIter) -> Result<Adapter, String> {
  let mut header = CsvHeader { records, row: None };
  let adapter = match kind {
    SourceType::Landr => wrap(Landr::new(&mut header)?),
    SourceType::Pretzel => wrap(Pretzel::new(&mut header)?),
    SourceType::RepostBySoundCloud => wrap(RepostBySoundCloud::new(&mut header)?),
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
      total_revenue: header.find_by_name("total_revenue)")?,
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
