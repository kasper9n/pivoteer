use bigdecimal::BigDecimal;
use chrono::{Datelike, NaiveDate};
use csv_pipeline::target::StdoutTarget;
use csv_pipeline::{Error, Pipeline, Transformer};
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::{env, fs};
use transformers::SumOrKeepTransform;

mod transformers;

type Settings = HashMap<String, Vec<String>>;
fn open_settings<P: Into<PathBuf>>(file_path: P) -> Vec<Source> {
	let file_path: PathBuf = file_path.into();
	let settings_str = fs::read_to_string(&file_path).unwrap();
	let settings: Settings = toml::from_str(&settings_str).unwrap();
	let settings_dir = file_path.parent().unwrap();
	let mut sources = Vec::new();
	for (store, file_paths) in settings {
		let kind = match store.as_str() {
			"landr" => SourceKind::Landr,
			"pretzel" => SourceKind::Pretzel,
			"pretzel_old_system" => SourceKind::PretzelOldSystem,
			"stem" => SourceKind::Stem,
			"symphonic" => SourceKind::Symphonic,
			"repost_network" => SourceKind::RepostNetwork,
			"repost_network_old" => SourceKind::RepostNetworkOld,
			_ => panic!("Unknown store: {}", store),
		};
		for file_path in file_paths {
			sources.push(Source {
				file_path: PathBuf::from(settings_dir).join(file_path),
				kind,
			});
		}
	}
	sources
}

struct Source {
	file_path: PathBuf,
	kind: SourceKind,
}
#[derive(Copy, Clone)]
enum SourceKind {
	Landr,
	Pretzel,
	PretzelOldSystem,
	Stem,
	Symphonic,
	RepostNetwork,
	RepostNetworkOld,
}

/// Get the reporting period that the date is in
fn reporting_period_of(date: &NaiveDate) -> String {
	date.with_day(1).unwrap().format("%Y-%m").to_string()
}

pub fn parse_date(s: &str, fmt: &str) -> Result<NaiveDate, chrono::ParseError> {
	NaiveDate::parse_from_str(s, fmt)
}

fn landr(file_path: &PathBuf) -> Pipeline {
	Pipeline::from_path(file_path)
		.unwrap()
		.validate(|headers, row| match headers.get_field(&row, "Share %") {
			Some("100") => Ok(()),
			Some(value) => Err(Error::InvalidField(value.to_string())),
			None => {
				let date_field = headers.get_field(&row, "Payment Date").unwrap();
				let date = parse_date(date_field, "%Y-%m-%d").unwrap();
				if date <= NaiveDate::from_ymd_opt(2022, 3, 8).unwrap() {
					Ok(())
				} else {
					Err(Error::MissingColumn("Share %".to_string()))
				}
			}
		})
		.add_col("Reporting Period", |headers, row| {
			let payment_date_s = headers.get_field(&row, "Payment Date").unwrap();
			let payment_date = parse_date(payment_date_s, "%Y-%m-%d");
			Ok(reporting_period_of(&payment_date.unwrap()))
		})
		.rename_col("Quantity of sales or streams", "Units")
		.rename_col("Net earnings (USD)", "Gross Royalties")
		.select(vec![
			"Reporting Period",
			"UPC",
			"ISRC",
			"Store",
			"Store service",
			"Units",
			"Gross Royalties",
		])
}

fn pretzel(file_path: &PathBuf) -> Pipeline {
	Pipeline::from_path(file_path)
		.unwrap()
		.add_col("Reporting Period", |headers, row| {
			let disbursement = headers.get_field(&row, "disbursement").unwrap();
			let disbursement = parse_date(&format!("1 {disbursement}"), "%d %b %y");
			Ok(reporting_period_of(&disbursement.unwrap()))
		})
		.add_col("UPC", |headers, row| {
			// UPCs are not always available for reports up to 2020. Pretzel had
			// an old and new report system in use at once, and the old one lacks UPCs
			let disbursement = headers.get_field(&row, "disbursement").unwrap();
			let disbursement = parse_date(&format!("1 {disbursement}"), "%d %b %y").unwrap();
			if disbursement.year() <= 2020 {
				let icpn = headers.get_field(&row, "icpn").unwrap_or_default();
				return Ok(icpn.to_string());
			}
			Ok(headers.get_field(&row, "icpn").unwrap().to_string())
		})
		.rename_col("isrc", "ISRC")
		.add_col("Store", |_headers, _row| Ok("Pretzel".to_string()))
		.add_col("Store service", |_headers, _row| Ok("Pretzel".to_string()))
		.add_col("Units", |headers, row| {
			let total_plays = headers
				.get_field(&row, "total_plays")
				.unwrap()
				.parse::<u16>()
				.unwrap();
			let downloads = match headers.get_field(&row, "downloads_count") {
				Some(downloads) => downloads.parse::<u16>().unwrap(),
				// does not exist in pre-2022 reports
				None => 0,
			};
			Ok((total_plays + downloads).to_string())
		})
		.rename_col("total_revenue", "Gross Royalties")
		.select(vec![
			"Reporting Period",
			"UPC",
			"ISRC",
			"Store",
			"Store service",
			"Units",
			"Gross Royalties",
		])
}

fn skip_to_new_header(reader: csv::Reader<File>, n: usize) -> csv::Reader<File> {
	let mut records = reader.into_records();
	let header = records.nth(n).unwrap().unwrap();
	let mut reader = records.into_reader();
	reader.set_headers(header);
	reader
}

fn repost_network(file_path: &PathBuf) -> Pipeline {
	let file = File::open(file_path).unwrap();
	let reader = csv::ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.from_reader(file);
	let reader = skip_to_new_header(reader, 2);
	Pipeline::from_reader(reader)
		.unwrap()
		.rename_col("Reporting Period", "Activity Period")
		.rename_col("Accounting Period", "Reporting Period")
		.rename_col("Partner", "Store")
		.rename_col("Type", "Store service")
		.rename_col("Revenue (USD)", "Gross Royalties")
		.select(vec![
			"Reporting Period",
			"UPC",
			"ISRC",
			"Store",
			"Store service",
			"Units",
			"Gross Royalties",
		])
}

fn repost_network_old(file_path: &PathBuf) -> Pipeline {
	Pipeline::from_path(file_path)
		.unwrap()
		.add_col("Reporting Period", |headers, row| {
			let accounting_period = headers.get_field(&row, "Accounting Period").unwrap();
			let accounting_period = parse_date(&format!("{accounting_period} 1"), "%Y %b %d");
			Ok(reporting_period_of(&accounting_period.unwrap()))
		})
		.add_col("Store", |_headers, _row| Ok("".to_string()))
		.add_col("Store service", |_headers, _row| Ok("".to_string()))
		.add_col("Units", |_headers, _row| Ok("".to_string()))
		.rename_col("Revenue (USD)", "Gross Royalties")
		.select(vec![
			"Reporting Period",
			"UPC",
			"ISRC",
			"Store",
			"Store service",
			"Units",
			"Gross Royalties",
		])
}

fn stem(file_path: &PathBuf) -> Pipeline {
	Pipeline::from_path(file_path)
		.unwrap()
		.add_col("Reporting Period", |headers, row| {
			let year = headers.get_field(&row, "ingest_year").unwrap();
			let month = headers.get_field(&row, "ingest_month").unwrap();
			let ingest_date = parse_date(&format!("{year}-{month}-1"), "%Y-%m-%d");
			Ok(reporting_period_of(&ingest_date.unwrap()))
		})
		.rename_col("upc", "UPC")
		.rename_col("isrc", "ISRC")
		.rename_col("platform", "Store")
		.rename_col("platform_detail", "Store service")
		.add_col("Units", |headers, row| {
			let downloads = headers.get_field(&row, "downloads").unwrap();
			let views = headers.get_field(&row, "views").unwrap();
			if downloads == "0" {
				Ok(downloads.to_string())
			} else if views == "0" {
				Ok(views.to_string())
			} else {
				Err(Error::InvalidField(format!(
					"Both downloads and views are non-zero: {} and {}",
					downloads, views
				)))
			}
		})
		.rename_col("net_royalties", "Gross Royalties")
		.select(vec![
			"Reporting Period",
			"UPC",
			"ISRC",
			"Store",
			"Store service",
			"Units",
			"Gross Royalties",
		])
}

fn symphonic(file_path: &PathBuf) -> Pipeline {
	fn symphonic_quarter(s: &str) -> Option<(u16, u8)> {
		// Q(1-4)(06-17)
		let re = Regex::new(r"^Q([1-4])(0[6-9]|1[0-7])$").ok()?;
		let captures = re.captures(s)?;
		let quarter: u8 = captures[1].parse().unwrap();
		let year_2_digit = 2000 + captures[2].parse::<u16>().unwrap();
		Some((year_2_digit, quarter))
	}

	Pipeline::from_path(file_path)
		.unwrap()
		.map_col("Reporting Period", |reporting_period| {
			if reporting_period.starts_with("Q") {
				match symphonic_quarter(reporting_period) {
					Some((year, quarter)) => {
						let date = NaiveDate::from_ymd_opt(year as i32, quarter as u32 * 3, 1);
						return Ok(reporting_period_of(&date.unwrap()));
					}
					None => return Err(Error::InvalidField(reporting_period.to_string())),
				};
			} else if reporting_period == "JAN-FEB-18" {
				return Ok("2018-01".to_string());
			} else {
				let date = parse_date(&format!("1-{reporting_period}"), "%d-%b-%y");
				Ok(reporting_period_of(&date.unwrap()))
			}
		})
		.rename_col("UPC Code", "UPC")
		.rename_col("ISRC Code", "ISRC")
		.rename_col("Digital Service Provider", "Store")
		.rename_col("Delivery", "Store service")
		.add_col("Units", |headers, row| {
			let count = headers.get_field(&row, "Count").unwrap();
			let is_void = headers.get_field(&row, "Sale or Void").unwrap() == "Void";
			if is_void && !count.starts_with('-') {
				Ok(format!("-{}", count))
			} else {
				Ok(count.to_string())
			}
		})
		.rename_col("Royalty ($US)", "Gross Royalties")
		.select(vec![
			"Reporting Period",
			"UPC",
			"ISRC",
			"Store",
			"Store service",
			"Units",
			"Gross Royalties",
		])
}

fn source(source: &Source) -> Pipeline<'_> {
	match &source.kind {
		SourceKind::Landr => landr(&source.file_path),
		SourceKind::Pretzel | SourceKind::PretzelOldSystem => pretzel(&source.file_path),
		SourceKind::RepostNetwork => repost_network(&source.file_path),
		SourceKind::RepostNetworkOld => repost_network_old(&source.file_path),
		SourceKind::Symphonic => symphonic(&source.file_path),
		SourceKind::Stem => stem(&source.file_path),
	}
}

fn main() {
	let settings_path = match env::args().nth(1) {
		Some(arg) => arg,
		None => {
			eprintln!("No Settings.json argument given");
			return;
		}
	};
	let sources = open_settings(&settings_path);

	let pipelines: Vec<_> = sources.iter().map(|x| source(&x)).collect();

	Pipeline::from_pipelines(pipelines)
		.transform_into(|| {
			vec![
				Transformer::new("Reporting Period").keep_unique(),
				Transformer::new("UPC").keep_unique(),
				Transformer::new("ISRC").keep_unique(),
				Transformer::new("Store").keep_unique(),
				Transformer::new("Store service").keep_unique(),
				Transformer::new("Units").sum_but_keep(0 as i64, ""),
				Transformer::new("Gross Royalties").sum(BigDecimal::from(0)),
			]
		})
		.flush(StdoutTarget::new())
		.collect_into_string()
		.unwrap();
}
