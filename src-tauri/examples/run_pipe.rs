use csv::{Reader, ReaderBuilder};
use pivoteer::pipeline::pipeline::Pipeline;
use std::fs::File;
use std::path::Path;

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
