use super::headers::Headers;
use super::pipe::{Pipe, PipeIterator};
use super::{Error, Row, RowResult};
use csv::Reader;
use std::fs::File;

pub struct Pipeline {
  pub headers: Headers,
  pipe: PipeIterator,
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
