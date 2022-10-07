use crate::cmd::ColumnLocation;
use crate::project::{ColumnType, SourceType};

pub trait Adapter {
  fn header_row_index(&self) -> usize;
  fn column_location(&self, kind: &ColumnType) -> &ColumnLocation;
}

pub fn adapter(source_type: &SourceType) -> impl Adapter {
  match source_type {
    SourceType::Landr => LANDR,
  }
}

struct Locations<'a> {
  isrc: ColumnLocation<'a>,
  upc: ColumnLocation<'a>,
  net_earnings: ColumnLocation<'a>,
}
struct BasicAdapter<'a> {
  header_row_index: usize,
  locations: Locations<'a>,
}
impl Adapter for BasicAdapter<'_> {
  fn header_row_index(&self) -> usize {
    self.header_row_index
  }
  fn column_location(&self, kind: &ColumnType) -> &ColumnLocation {
    match kind {
      ColumnType::Isrc => &self.locations.isrc,
      ColumnType::Upc => &self.locations.upc,
      ColumnType::NetEarnings => &self.locations.net_earnings,
    }
  }
}

const LANDR: BasicAdapter = BasicAdapter {
  header_row_index: 0,
  locations: Locations {
    isrc: ColumnLocation::Name("ISRC"),
    upc: ColumnLocation::Name("UPC"),
    net_earnings: ColumnLocation::Name("Net earnings (USD)"),
  },
};
