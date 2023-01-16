use bigdecimal::BigDecimal;
use csv_pipeline::{Error, Pipeline, Transformer};

// fn read_csvs<P: AsRef<Path>>(file_paths: Vec<P>) -> Vec<InputStream> {
//   file_paths
//     .iter()
//     .map(|file_path| read_csv(file_path))
//     .collect()
// }

// fn read_folder<P: AsRef<Path>>(folder_path: P) -> Vec<InputStream> {
//   let file_paths = read_dir(folder_path)
//     .unwrap()
//     .map(|entry| entry.unwrap().path())
//     .collect();
//   read_csvs(file_paths)
// }

fn main() {
	let csv = Pipeline::from_path("/Volumes/GoogleDrive/Shared drives/Lacuna/Financial/Sales Reports/Landr/earnings-report-2022-9.csv")
  .unwrap()
  .validate_col("Share %", |pct| match pct {
    "100" => Ok(()),
    _ => Err(Error::InvalidField("Share % is not 100".to_string())),
  })
  .transform_into(|| vec![
    Transformer::new("UPC").keep_unique(),
    Transformer::new("ISRC").keep_unique(),
    Transformer::new("Store").keep_unique(),
    Transformer::new("Store service").keep_unique(),
    Transformer::new("Sales or streams").from_col("Quantity of sales or streams").sum(0 as u64),
    Transformer::new("Royalties").from_col("Net earnings (USD)").sum(BigDecimal::from(0))
    // Transformer::new("Royalties").from_col("Net earnings (USD)").reduce(
    //   |accumulator, current| {
    //     let score: f64 = current.parse().unwrap();
    //     Ok(accumulator + score)
    //   },
    //   0.0,
    // ),
  ])
  .collect_into_string()
  .unwrap();

	println!("{csv}");
}
