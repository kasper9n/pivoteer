use bigdecimal::BigDecimal;
use csv::{ReaderBuilder, StringRecord};
use csvsc::error::{Error as CsvscError, Result as CsvscResult, RowResult};
use csvsc::input::{InputStreamBuilder, ReaderSource};
use csvsc::{Headers, InputStream, Reducer, Row, RowStream, Target};
use std::fs::read_dir;
use std::path::Path;
use std::str::FromStr;

fn str_to_dec<'a>(input: &'a str) -> Result<BigDecimal, String> {
  return match BigDecimal::from_str(&input) {
    Ok(num) => Ok(num),
    Err(e) => Err(format!("Invalid number: {}", e.to_string())),
  };
}

fn read_csv<P: AsRef<Path>>(file_path: P) -> InputStream {
  let ext = file_path.as_ref().extension().unwrap_or_default();
  let delimiter = match ext.to_string_lossy().as_ref() {
    "tsv" => b'\t',
    "csv" => b',',
    _ => panic!("Unsupported file {}", file_path.as_ref().display()),
  };
  let reader: ReaderSource = ReaderBuilder::new()
    .delimiter(delimiter)
    .from_path(file_path)
    .unwrap()
    .into();
  InputStreamBuilder::from_readers(vec![reader])
    .build()
    .unwrap()
}

fn read_csvs<P: AsRef<Path>>(file_paths: Vec<P>) -> Vec<InputStream> {
  file_paths
    .iter()
    .map(|file_path| read_csv(file_path))
    .collect()
}

fn read_folder<P: AsRef<Path>>(folder_path: P) -> Vec<InputStream> {
  let file_paths = read_dir(folder_path)
    .unwrap()
    .map(|entry| entry.unwrap().path())
    .collect();
  read_csvs(file_paths)
}

fn parse_landr_csv(chain: InputStream) {
  println!(
    "Contains Split %: {}",
    chain.headers().contains_key("Split %")
  );

  // let chain = AddIf::new(chain, "Share %", |headers, row| {
  //   // let payment_date = headers.get_field(row, "Payment Date").unwrap();
  //   // match payment_date {
  //   //   "2021-09-30" => Ok(net_earnings.to_string()),
  //   //   _ => Err(csvsc::Error::InvalidFormat(
  //   //     "Payment Date is not 2021-09-30".to_string(),
  //   //   )),
  //   // }
  //   Ok("100".to_owned())
  // });
}

pub fn main() -> Result<(), String> {
  // let readers = read_csvs(vec![
  //   "/Volumes/GoogleDrive/Shared drives/Lacuna/Financial/Sales Reports/Landr/earnings-report-2022-9.csv",
  //   "/Volumes/GoogleDrive/Shared drives/Lacuna/Financial/Sales Reports/Landr/earnings-report-2022-8.csv",
  //   "/Volumes/GoogleDrive/Shared drives/Lacuna/Financial/Sales Reports/Landr/earnings-report-2022-7.csv",
  // ]);
  let mut chains =
    read_folder("/Volumes/GoogleDrive/Shared drives/Lacuna/Financial/Sales Reports/Landr");

  // let chain: Vec<_> =
  //   read_folder("/Volumes/GoogleDrive/Shared drives/Lacuna/Financial/Sales Reports/Landr")
  //     .into_iter()
  //     .map(|chain| parse_landr_csv(chain))
  //     .collect();
  let chain0 = chains.remove(0);

  let mut chain = chain0
    .map_col("Share %", |share_pct| match share_pct {
      "100" => Ok(share_pct.to_string()),
      _ => Err(csvsc::Error::InvalidFormat(format!("Share % is not 100"))),
    })
    .add_with("Gross Royalties (USD)", |headers, row| {
      let net_earnings = headers.get_field(row, "Net earnings (USD)").unwrap();
      let share_pct = headers.get_field(row, "Share %");
      match share_pct {
        Some("100") => Ok(net_earnings.to_string()),
        _ => Err(csvsc::Error::InvalidFormat(
          "Share % is not 100".to_string(),
        )),
      }
    })
    .unwrap()
    .group(["UPC", "ISRC", "Store", "Store service"], |row_stream| {
      row_stream
        // Payment Date,Start of reporting period,End of reporting period,Country of sale or stream
        .reduce(vec![
          Reducer::with_name("UPC").of_column("UPC").last("").unwrap(),
          Reducer::with_name("ISRC")
            .of_column("ISRC")
            .last("")
            .unwrap(),
          Reducer::with_name("Store")
            .of_column("Store")
            .last("")
            .unwrap(),
          Reducer::with_name("Store service")
            .of_column("Store service")
            .last("")
            .unwrap(),
          Reducer::with_name("Sales/streams")
            .of_column("Quantity of sales or streams")
            .sum(0 as u64)
            .unwrap(),
          Reducer::with_name("Gross Royalties (USD)")
            .of_column("Gross Royalties (USD)")
            .with_closure(
              |acc, cur| Ok(acc + str_to_dec(cur).unwrap()),
              BigDecimal::from(0),
            )
            .unwrap(),
        ])
        .unwrap()
    })
    .map_row(
      |_headers, row| {
        // Go creative here in the creation of your new row(s)
        Ok(vec![Ok(row.clone())].into_iter())
      },
      |old_headers| {
        // be responsible and provide proper headers from the old ones
        old_headers.clone()
      },
    )
    // .map_row(|headers, row| Ok(row), |headers| Ok(headers))
    .flush(Target::path("output.csv"))
    .unwrap()
    .into_iter();

  while let Some(item) = chain.next() {
    match item {
      Err(e) => eprintln!("Error: {:?}", e),
      Ok(_) => {}
    };
  }

  Ok(())
}
