use csv::{Reader, ReaderBuilder};
use std::collections::HashMap;
use std::fs::File;
use std::iter::Map;
use std::path::Path;

/// An error found somewhere in the transformation chain.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
  DuplicatedColumn(String),
  Csv,
}

pub type RowResult = Result<Row, Error>;
// pub type Pipe = std::result::IntoIter<RowResult>;
pub type Pipe = Box<dyn Iterator<Item = RowResult>>;

pub struct Pipeline {
  headers: Headers,
  // pipe: Box<dyn IntoIterator<Item = RowResult, IntoIter = Pipe>>,
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

  // pub fn add_col<'a, F>(mut self, name: &str, get_value: &'a mut F) -> Self
  // where
  //   Self: 'a,
  //   F: FnMut(&Headers, &Row) -> Result<String, Error> + 'a,
  // {
  //   // pub fn add_col(mut self, name: &str, mut get_value: F) -> Self {
  //   self.headers.add(name);
  //   let headers = self.headers.clone();
  //   let newpipe = self.pipe.map(move |row_result| {
  //     let mut row = row_result?;
  //     let value = get_value(&headers, &row)?;
  //     row.push_field(&value);
  //     Ok(row)
  //   });
  //   self.pipe = Box::new(newpipe);
  //   self
  // }

  pub fn add_col_t<F>(mut self, name: &str, mut get_value: F) -> Self
  where
    F: FnMut(&Headers, &Row) -> Result<String, Error>,
  {
    // // pub fn add_col(mut self, name: &str, mut get_value: F) -> Self {
    // self.headers.add(name);
    // // let headers = self.headers.clone();
    // let add_col = StatefulIterator {
    //   state: get_value,
    //   iterator: self.pipe,
    // };
    // let iterator = add_col.map(|(row_result, state)| {
    //   return row_result;
    // });
    // self.pipe = Box::new(iterator);

    struct State<F> {
      get_value: F,
      headers: Headers,
    }

    let stateful_iterator = StatefulIterator {
      iterator: self.pipe,
      state: State {
        get_value,
        headers: self.headers.clone(),
      },
      f: |row_result, state| {
        let mut row = row_result?;
        let value = (state.get_value)(&state.headers, &row)?;
        row.push_field(&value);
        Ok(row)
      },
    };

    self.headers.add(name);

    // let x = stateful_iterator.map(|row_result, state| -> RowResult {
    //   let mut row = row_result?;
    //   let value = (state.get_value)(&state.headers, &row)?;
    //   row.push_field(&value);
    //   Ok(row)
    // });
    self.pipe = Box::new(stateful_iterator.iterator);

    // let newpipe = add_col_transformer.iterator.map(|row_result| {
    //   let mut row = row_result?;
    //   let value = (add_col_transformer.state)(&headers, &row)?;
    //   row.push_field(&value);
    //   Ok(row)
    // });
    // add_col_transformer.iterator = Box::new(newpipe);
    // self.pipe = Box::new(add_col_transformer);
    self
  }
}

pub struct StatefulIterator<I: Iterator, S, F: FnMut(I::Item, &mut S) -> I::Item> {
  iterator: I,
  state: S,
  f: F,
}
impl<I: Iterator, S, F> StatefulIterator<I, S, F>
where
  F: FnMut(I::Item, &mut S) -> I::Item,
{
  pub fn new(iterator: I, state: S, f: F) -> Self {
    Self { state, iterator, f }
  }

  // pub fn build<F>(mut self, mut f: F) -> Map<I, impl FnMut(I::Item) -> I::Item>
  // where
  //   F: FnMut(I::Item, &mut S) -> I::Item,
  // {
  //   self.iterator.map(move |item| {
  //     let y = f(item, &mut self.state);
  //     return y;
  //   })
  // }

  // pub fn map<O, F, R>(mut self, mut f: F) -> Map<I, impl FnMut(I::Item) -> O>
  // where
  //   F: FnMut(I::Item, &mut S) -> O,
  //   R: FnMut(I::Item) -> O,
  // {
  //   self.iterator.map(move |item| {
  //     let y = f(item, &mut self.state);
  //     return y;
  //   })
  // }
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
    pipeline = pipeline.add_col_t("Share %", |_, _| Ok("100".to_string() + &x));
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
