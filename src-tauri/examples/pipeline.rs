use csv::{Reader, ReaderBuilder};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

/// An error found somewhere in the transformation chain.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
  DuplicatedColumn(String),
  Csv,
}

pub type Row = csv::StringRecord;
pub type RowResult = Result<Row, Error>;
type PipeIterator = dyn Iterator<Item = RowResult>;
// pub type Pipe = Box<PipeIterator>;

pub struct Pipeline {
  headers: Headers,
  pipe: Box<PipeIterator>,
}

impl Pipeline {
  pub fn from_reader(mut reader: Reader<File>) -> Self {
    let headers_row = reader.headers().unwrap().clone();
    let records = reader.into_records().map(|r| {
      let row_result: RowResult = match r {
        Ok(row) => Ok(row),
        Err(err) => Err(Error::Csv),
      };
      row_result
    });
    Self {
      headers: Headers::from(headers_row),
      pipe: Box::new(records),
    }
  }

  pub fn add_col<F>(mut self, name: &str, get_value: F) -> Self
  where
    F: FnMut(&Headers, &Row) -> Result<String, Error>,
  {
    self.headers.add(name);

    struct State<F> {
      get_value: F,
      headers: Headers,
    }
    let pipe = Pipe::new(self.pipe).with_state(State {
      get_value,
      headers: self.headers.clone(),
    });
    let newpipe = pipe.map(|row_result, state| {
      let mut row = row_result?;
      let value = (state.get_value)(&state.headers, &row)?;
      row.push_field(&value);
      Ok(row)
    });

    self.pipe = Box::new(newpipe.iterator);

    self
  }
}

pub struct Pipe {
  iterator: Box<PipeIterator>,
}
impl Pipe {
  pub fn new(iterator: Box<PipeIterator>) -> Self {
    Self {
      iterator: Box::new(iterator.into_iter()),
    }
  }
  pub fn with_state<S>(self, state: S) -> StatefulPipeBuilder<S> {
    StatefulPipeBuilder::new(self.iterator, state)
  }
}
impl Iterator for Pipe {
  type Item = RowResult;

  fn next(&mut self) -> Option<Self::Item> {
    self.iterator.next()
  }
}

pub struct StatefulPipeBuilder<S> {
  iterator: Box<PipeIterator>,
  state: S,
}
impl<S> StatefulPipeBuilder<S> {
  pub fn new(iterator: Box<PipeIterator>, state: S) -> Self {
    Self { state, iterator }
  }
  pub fn map<F>(self, f: F) -> StatefulPipe<S, F>
  where
    F: FnMut(RowResult, &mut S) -> RowResult,
  {
    StatefulPipe {
      iterator: self.iterator,
      state: self.state,
      f,
    }
  }
}

pub struct StatefulPipe<S, F: FnMut(RowResult, &mut S) -> RowResult> {
  iterator: Box<PipeIterator>,
  state: S,
  f: F,
}
impl<S, F> Iterator for StatefulPipe<S, F>
where
  F: FnMut(RowResult, &mut S) -> RowResult,
{
  type Item = RowResult;

  fn next(&mut self) -> Option<Self::Item> {
    match self.iterator.next() {
      Some(item) => Some((self.f)(item, &mut self.state)),
      None => None,
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Headers {
  indexes: HashMap<String, usize>,
  names: Row,
}
impl Headers {
  pub fn add(&mut self, name: &str) -> bool {
    if self.indexes.contains_key(name) {
      return false;
    }

    self.names.push_field(name);
    self.indexes.insert(name.to_string(), self.names.len() - 1);

    true
  }

  pub fn contains(&self, name: &str) -> bool {
    self.indexes.contains_key(name)
  }
}
impl From<Row> for Headers {
  fn from(row: Row) -> Headers {
    Headers {
      indexes: row
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.to_string(), index))
        .collect(),
      names: row,
    }
  }
}

fn main() {
  let reader = read_csv("/Volumes/GoogleDrive/Shared drives/Lacuna/Financial/Sales Reports/Landr/earnings-report-2022-9.csv");
  let mut pipeline = Pipeline::from_reader(reader);
  let x = "100".to_string();
  if !pipeline.headers.contains("Share %") {
    pipeline = pipeline.add_col("Share %", |_, _| Ok("100".to_string() + &x));
    panic!("Missing Share %");
  }
}

fn read_csv<P: AsRef<Path>>(file_path: P) -> Reader<File> {
  let ext = file_path.as_ref().extension().unwrap_or_default();
  let delimiter = match ext.to_string_lossy().as_ref() {
    "tsv" => b'\t',
    "csv" => b',',
    _ => panic!("Unsupported file {}", file_path.as_ref().display()),
  };
  ReaderBuilder::new()
    .delimiter(delimiter)
    .from_path(file_path)
    .unwrap()
}
