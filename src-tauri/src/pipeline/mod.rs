pub mod headers;
pub mod pipe;
pub mod pipeline;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
  DuplicatedColumn(String),
  Csv,
}

pub type Row = csv::StringRecord;
pub type RowResult = Result<Row, Error>;
