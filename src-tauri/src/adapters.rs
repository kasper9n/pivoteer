use crate::cmd::get_cell;
use crate::project::{ColumnConfig, ColumnType, PeriodColumnConfig, SourceConfig, SourceType};
use crate::throw;
use chrono::{Datelike, NaiveDate};
use csv::{ReaderBuilder, StringRecord};
use std::fs::File;

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

pub type CsvIter = csv::StringRecordsIntoIter<File>;
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
  pub fn extract_from_records(records: &'a mut CsvIter) -> Result<Self, String> {
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

fn parse_date(s: &str, fmt: &str) -> Result<NaiveDate, String> {
  match NaiveDate::parse_from_str(s, fmt) {
    Ok(dt) => Ok(dt),
    Err(e) => throw!("Invalid date: {}", e.to_string()),
  }
}
fn get_quarter(date: &NaiveDate) -> u8 {
  match date.month() {
    1..=3 => 1,
    4..=6 => 2,
    7..=9 => 3,
    10..=12 => 4,
    _ => panic!("Invalid month {}", date.month()),
  }
}
fn new_period(year: i32, quarter: u8) -> String {
  format!("{:04}-Q{}", year, quarter)
}
fn get_period(date: &NaiveDate) -> String {
  new_period(date.year(), get_quarter(&date))
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
  payment_date: usize,
  isrc: usize,
  upc: usize,
  net_earnings: usize,
}
impl Landr {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      payment_date: header.position("Payment Date")?,
      isrc: header.position("ISRC")?,
      upc: header.position("UPC")?,
      net_earnings: header.position("Net earnings (USD)")?,
    })
  }
}
impl AdapterT for Landr {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Period => {
        let date = parse_date(&row.get(self.payment_date)?, "%Y-%m-%d")?;
        Ok(get_period(&date))
      }
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.net_earnings),
    }
  }
}

struct Pretzel {
  disbursement_month: usize,
  isrc: usize,
  total_revenue: usize,
}
impl Pretzel {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      disbursement_month: header.position("disbursement")?,
      isrc: header.position("isrc")?,
      total_revenue: header.position("total_revenue")?,
    })
  }
}
impl AdapterT for Pretzel {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Period => {
        let date = parse_date(&row.get(self.disbursement_month)?, "%b %y")?;
        Ok(get_period(&date))
      }
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => Ok("".to_owned()),
      ColumnType::NetEarnings => row.get(self.total_revenue),
    }
  }
}

struct RepostBySoundCloud {
  accounting_period: usize,
  isrc: usize,
  upc: usize,
  revenue: usize,
}
impl RepostBySoundCloud {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    header.skip_rows(2)?;
    Ok(Self {
      accounting_period: header.position("Accounting Period")?,
      isrc: header.position("ISRC")?,
      upc: header.position("UPC")?,
      revenue: header.position("Revenue (USD)")?,
    })
  }
}
impl AdapterT for RepostBySoundCloud {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Period => {
        let date = parse_date(&row.get(self.accounting_period)?, "%Y-%m")?;
        Ok(get_period(&date))
      }
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.revenue),
    }
  }
}

struct Stem {
  report_year: usize,
  report_month: usize,
  isrc: usize,
  upc: usize,
  net_royalties: usize,
}
impl Stem {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      report_year: header.position("report_year")?,
      report_month: header.position("report_month")?,
      isrc: header.position("isrc")?,
      upc: header.position("upc")?,
      net_royalties: header.position("net_royalties")?,
    })
  }
}
impl AdapterT for Stem {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Period => {
        let year = row.get(self.report_year)?;
        let month = row.get(self.report_month)?;
        let date = parse_date(&format!("{:04}-{}", year, month), "%Y-%m")?;
        Ok(get_period(&date))
      }
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.net_royalties),
    }
  }
}

struct Symphonic {
  reporting_period: usize,
  isrc: usize,
  upc: usize,
  net_royalties: usize,
}
impl Symphonic {
  fn new(header: &mut CsvHeader) -> Result<Self, String> {
    Ok(Self {
      reporting_period: header.position("Reporting Period")?,
      isrc: header.position("ISRC Code")?,
      upc: header.position("UPC Code")?,
      net_royalties: header.position("Royalty ($US)")?,
    })
  }
}
impl AdapterT for Symphonic {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    match kind {
      ColumnType::Period => {
        let reporting_period = row.get(self.reporting_period)?;

        if reporting_period.starts_with("Q") {
          let q: u8 = match &reporting_period[..2] {
            "Q1" => 1,
            "Q2" => 2,
            "Q3" => 3,
            "Q4" => 4,
            _ => throw!("Invalid reporting period: {}", reporting_period),
          };
          let yeardate = match NaiveDate::parse_from_str(&reporting_period[2..], "%y") {
            Ok(date) => date,
            Err(_) => throw!("Invalid reporting period: {}", reporting_period),
          };
          return Ok(new_period(yeardate.year(), q));
        }

        if reporting_period == "JAN-FEB-18" {
          return Ok("2018-Q1".to_owned());
        }

        let date = parse_date(&reporting_period, "%b-%y")?;
        Ok(get_period(&date))
      }
      ColumnType::Isrc => row.get(self.isrc),
      ColumnType::Upc => row.get(self.upc),
      ColumnType::NetEarnings => row.get(self.net_royalties),
    }
  }
}

pub struct CustomSource {
  pub period: Option<PeriodColumn>,
  pub isrc: Option<Column>,
  pub upc: Option<Column>,
  pub revenue: Option<Column>,
}
impl CustomSource {
  fn new(header: &mut CsvHeader, config: SourceConfig) -> Result<Self, String> {
    header.skip_rows(config.header_row_index)?;
    Ok(Self {
      period: PeriodColumn::from_config(config.period, &header.row)?,
      isrc: Column::from_config(config.isrc, &header.row)?,
      upc: Column::from_config(config.upc, &header.row)?,
      revenue: Column::from_config(config.revenue, &header.row)?,
    })
  }
}
impl AdapterT for CustomSource {
  fn get(&self, row: &CsvRow, kind: &ColumnType) -> Result<String, String> {
    let column_field = match &kind {
      ColumnType::Period => self.period.as_ref().map(|period| &period.column),
      ColumnType::Isrc => self.isrc.as_ref(),
      ColumnType::Upc => self.upc.as_ref(),
      ColumnType::NetEarnings => self.revenue.as_ref(),
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

pub struct PeriodColumn {
  pub column: Column,
  pub format: String,
}
impl PeriodColumn {
  fn from_config(
    config: Option<PeriodColumnConfig>,
    header: &CsvRow,
  ) -> Result<Option<Self>, String> {
    match config {
      Some(config) => {
        let column = Column::from_config_required(config.column, header)?;
        Ok(Some(Self {
          column,
          format: config.format,
        }))
      }
      None => Ok(None),
    }
  }
}

pub enum Column {
  Index(usize),
  CustomValue(String),
}
impl Column {
  fn from_config(config: Option<ColumnConfig>, header: &CsvRow) -> Result<Option<Self>, String> {
    match config {
      Some(config) => Ok(Some(Self::from_config_required(config, header)?)),
      None => Ok(None),
    }
  }
  fn from_config_required(config: ColumnConfig, header: &CsvRow) -> Result<Self, String> {
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
    Ok(column)
  }
}
