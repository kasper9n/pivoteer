use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use csv_pipeline::{Error, Pipeline, Transformer};
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize)]
struct Settings {
	landr: Vec<String>,
}
impl Settings {
	fn open(file_path: &str) -> Self {
		let file = File::open(file_path).unwrap();
		let reader = BufReader::new(file);
		let settings = serde_json::from_reader(reader).unwrap();
		settings
	}
}

fn landr(file_path: &str) -> Pipeline {
	Pipeline::from_path(file_path)
		.unwrap()
		.validate(|headers, row| match headers.get_field(&row, "Share %") {
			Some("100") => Ok(()),
			Some(_) => Err(Error::InvalidField("Share % is not 100".to_string())),
			None => {
				let date_field = headers.get_field(&row, "Payment Date").unwrap();
				let date = NaiveDate::parse_from_str(date_field, "%Y-%m-%d").unwrap();
				if date <= NaiveDate::from_ymd_opt(2022, 3, 8).unwrap() {
					Ok(())
				} else {
					Err(Error::MissingColumn("Share %".to_string()))
				}
			}
		})
		.add_col("Period", |headers, row| {
			let payment_date = headers.get_field(&row, "Payment Date").unwrap();
			let date = NaiveDate::parse_from_str(payment_date, "%Y-%m-%d").unwrap();
			let period = date.format("%Y-%m").to_string();
			Ok(period)
		})
		.rename_col("Quantity of sales or streams", "Sales or streams")
		.rename_col("Net earnings (USD)", "Gross Royalties")
		.select(vec![
			"Period",
			"UPC",
			"ISRC",
			"Store",
			"Store service",
			"Sales or streams",
			"Gross Royalties",
		])
}

fn main() {
	let settings_path = match env::args().nth(1) {
		Some(arg) => arg,
		None => {
			eprintln!("No Settings.json argument given");
			return;
		}
	};
	let settings = Settings::open(&settings_path);

	let lands_pipelines: Vec<_> = settings.landr.iter().map(|x| landr(&x)).collect();

	let csv = Pipeline::from_pipelines(lands_pipelines)
		.transform_into(|| {
			vec![
				Transformer::new("Period").keep_unique(),
				Transformer::new("UPC").keep_unique(),
				Transformer::new("ISRC").keep_unique(),
				Transformer::new("Store").keep_unique(),
				Transformer::new("Store service").keep_unique(),
				Transformer::new("Sales or streams").sum(0 as i64),
				Transformer::new("Gross Royalties").sum(BigDecimal::from(0)),
			]
		})
		.collect_into_string()
		.unwrap();

	println!("{csv}");
}
