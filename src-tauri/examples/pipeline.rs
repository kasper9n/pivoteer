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

pub type RowResult = Result<Row, Error>;
pub type Pipe = Box<dyn Iterator<Item = RowResult>>;

pub struct Pipeline {
  headers: Headers,
  pipe: Pipe,
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
    let state = State {
      get_value,
      headers: self.headers.clone(),
    };
    let stateful_pipe = StatefulPipe::new(self.pipe, state);
    let newpipe = stateful_pipe.map(|row_result, state| {
      let mut row = row_result?;
      let value = (state.get_value)(&state.headers, &row)?;
      row.push_field(&value);
      Ok(row)
    });

    self.pipe.with_state(state);

    // let stateful_iterator =
    //   StatefulIteratorBuilder::new(self.pipe, state).map(|row_result, state| {
    //     let mut row = row_result?;
    //     let value = (state.get_value)(&state.headers, &row)?;
    //     row.push_field(&value);
    //     Ok(row)
    //   });

    self.pipe = Box::new(newpipe);

    self
  }
}
pub struct StatefulPipe<I: Iterator, S> {
  iterator: I,
  state: S,
}
impl<I: Iterator, S> StatefulPipe<I, S> {
  pub fn new(iterator: I, state: S) -> Self {
    Self { state, iterator }
  }
  pub fn map<F>(self, f: F) -> StatefulIterator<I, S, F>
  where
    F: FnMut(I::Item, &mut S) -> I::Item,
  {
    StatefulIterator {
      iterator: self.iterator,
      state: self.state,
      f,
    }
  }
}

pub struct StatefulIterator<I: Iterator, S, F: FnMut(I::Item, &mut S) -> I::Item> {
  iterator: I,
  state: S,
  f: F,
}
impl<I: Iterator, S, F> Iterator for StatefulIterator<I, S, F>
where
  F: FnMut(I::Item, &mut S) -> I::Item,
{
  type Item = I::Item;

  fn next(&mut self) -> Option<Self::Item> {
    match self.iterator.next() {
      Some(item) => Some((self.f)(item, &mut self.state)),
      None => None,
    }
  }
}

pub type Row = csv::StringRecord;

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
