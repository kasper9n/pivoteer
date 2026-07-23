use bigdecimal::BigDecimal;
use chrono::{Datelike, NaiveDate};
use csv_pipeline::{Error, Pipeline};
use regex::Regex;
use std::path::PathBuf;
use std::{fs::File, path::Path};

use crate::manifest::{AccountingPeriodManifest, SourceManifest};

/// Get the reporting period that the date is in
pub fn reporting_period_of(date: &NaiveDate) -> String {
	date.with_day(1).unwrap().format("%Y-%m").to_string()
}

pub fn parse_date(s: &str, fmt: &str) -> Result<NaiveDate, chrono::ParseError> {
	NaiveDate::parse_from_str(s, fmt)
}

#[derive(Clone)]
pub struct Source {
	pub file_path: PathBuf,
	pub kind: SourceKind,
	pub eur_usd_rate: Option<BigDecimal>,
}
impl Source {
	pub fn from_manifest(manifest: &AccountingPeriodManifest, dir: &PathBuf) -> Vec<Self> {
		manifest
			.sources_by_platform
			.iter()
			.map(|(platform, file_paths)| {
				let kind = SourceKind::from_str(platform);
				let sources = file_paths
					.into_iter()
					.map(|source_info| {
						let source = match source_info {
							SourceManifest::Path(file_path) => Source {
								file_path: PathBuf::from(dir).join(file_path),
								kind,
								eur_usd_rate: None,
							},
							SourceManifest::FullSource(source_info) => Source {
								file_path: PathBuf::from(dir).join(&source_info.path),
								kind,
								eur_usd_rate: source_info.eur_usd_rate.clone(),
							},
						};
						if !Path::exists(&source.file_path) {
							panic!(
								"File not found: {:?}. From {} {}",
								source.file_path,
								manifest.name.to_string(),
								platform
							);
						}
						source
					})
					.collect::<Vec<_>>();
				sources
			})
			.flatten()
			.collect()
	}
	pub fn process_source(&self) -> Pipeline<'_> {
		match &self.kind {
			SourceKind::Bandcamp => bandcamp(&self.file_path),
			SourceKind::CurveRoyaltySystems => curve(&self),
			SourceKind::Landr => landr(&self.file_path),
			SourceKind::Pretzel | SourceKind::PretzelOldSystem => pretzel(&self.file_path),
			SourceKind::RepostNetwork => repost_network(&self.file_path),
			SourceKind::Stem => stem(&self.file_path),
			SourceKind::Symphonic => symphonic(&self.file_path),
		}
	}
}

#[derive(Copy, Clone)]
pub enum SourceKind {
	Bandcamp,
	CurveRoyaltySystems,
	Landr,
	Pretzel,
	PretzelOldSystem,
	RepostNetwork,
	Stem,
	Symphonic,
}
impl SourceKind {
	pub fn from_str(platform_str: &str) -> Self {
		match platform_str {
			"bandcamp" => SourceKind::Bandcamp,
			"frequency" => SourceKind::CurveRoyaltySystems,
			"landr" => SourceKind::Landr,
			"pretzel_old_system" => SourceKind::PretzelOldSystem,
			"pretzel" => SourceKind::Pretzel,
			"repost_network" => SourceKind::RepostNetwork,
			"stem" => SourceKind::Stem,
			"symphonic" => SourceKind::Symphonic,
			_ => panic!("Unknown platform: {}", platform_str),
		}
	}
}

fn bandcamp(file_path: &PathBuf) -> Pipeline<'_> {
	Pipeline::from_path(file_path)
		.unwrap()
		.filter_col("item type", |item_type| match item_type {
			"track" | "album" => true,
			"payout" => false,
			_ => panic!("Unknown Bandcamp item type \"{item_type}\""),
		})
		.add_col("Reporting Period", |headers, row| {
			let date = headers.get_field(&row, "date").unwrap();
			let date = parse_date(date, "%m/%d/%y %I:%M%p").unwrap();
			Ok(reporting_period_of(&date))
		})
		.rename_col("upc", "UPC")
		.rename_col("isrc", "ISRC")
		.add_col("Store", |_headers, _rows| Ok("Bandcamp".to_string()))
		.add_col("Store service", |_h, _r| Ok("Bandcamp".to_string()))
		.rename_col("quantity", "Units")
		.rename_col("net amount", "Gross Royalties")
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

fn curve(source: &Source) -> Pipeline<'_> {
	Pipeline::from_path(&source.file_path)
		.unwrap()
		// These are probably ok, but handle them later
		.validate_col("Type", |value| match value {
			"Track" => Ok(()),
			value => Err(Error::InvalidField(value.to_string())),
		})
		.validate_col("Release Version", |value| match value {
			"" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Release Version: {value}"
			))),
		})
		.validate_col("Release Label", |value| match value {
			"" | "Frequency Music" | "Frequency Music / Lacuna Media" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Release Label: {value}"
			))),
		})
		.validate_col("PPD", |value| match value {
			"0" => Ok(()),
			value => Err(Error::InvalidField(format!("Unxpected PPD: {value}"))),
		})
		.validate_col("Track Version", |value| match value {
			"" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Track Version: {value}"
			))),
		})
		.validate_col("Distribution Channel", |value| match value {
			"Digital" | "Neighbouring Rights" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Distribution Channel: {value}"
			))),
		})
		.validate_col("Price Category", |value| match value {
			"" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Price Category: {value}"
			))),
		})
		.validate_col("Calculation Type", |value| match value {
			"Net Receipts" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Calculation Type: {value}"
			))),
		})
		.validate_col("Mechanical Deduction Type", |value| match value {
			"" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Mechanical Deduction Type: {value}"
			))),
		})
		.validate_col("Mechanical Deduction Rate", |value| match value {
			"100" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Mechanical Deduction Rate: {value}"
			))),
		})
		.validate_col("Mechanical Deduction", |value| match value {
			"0" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Mechanical Deduction: {value}"
			))),
		})
		.validate_col("Deduction Unit Rate", |value| match value {
			"" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Deduction Unit Rate: {value}"
			))),
		})
		.validate_col("Deduction Rate", |value| match value {
			"0" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Deduction Rate: {value}"
			))),
		})
		.validate_col("Deduction", |value| match value {
			"0" => Ok(()),
			value => Err(Error::InvalidField(format!("Unxpected Deduction: {value}"))),
		})
		.validate_col("Unit Rate", |value| match value {
			"" => Ok(()),
			value => Err(Error::InvalidField(format!("Unxpected Unit Rate: {value}"))),
		})
		.validate_col("Multiplier", |value| match value {
			"1" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Multiplier: {value}"
			))),
		})
		.validate_col("Reduction Rate", |value| match value {
			"" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Reduction Rate: {value}"
			))),
		})
		.validate_col("Reserve Rate", |value| match value {
			"0" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Reserve Rate: {value}"
			))),
		})
		.validate_col("Reserve", |value| match value {
			"0" => Ok(()),
			value => Err(Error::InvalidField(format!("Unxpected Reserve: {value}"))),
		})
		.validate_col("Specific Withholding Tax Rate", |value| match value {
			"" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Specific Withholding Tax Rate: {value}"
			))),
		})
		.validate_col("Specific Withholding Tax", |value| match value {
			"0" => Ok(()),
			value => Err(Error::InvalidField(format!(
				"Unxpected Specific Withholding Tax: {value}"
			))),
		})
		.filter_col("Participation Rate", |participation_rate| {
			participation_rate == "100"
		})
		.add_col("Reporting Period", |headers, row| {
			// This isn't actually the reporting period, just the date when the sale was settled
			let transaction_date_s = headers.get_field(&row, "Transaction Date").unwrap();
			let transaction_date = parse_date(transaction_date_s, "%Y-%m-%d");
			Ok(reporting_period_of(&transaction_date.unwrap()))
		})
		.rename_col("Barcode", "UPC")
		.rename_col("Source", "Store")
		.rename_col("Configuration", "Store service")
		.add_col("Gross Royalties", |headers, row| {
			let currency = headers.get_field(&row, "Currency").unwrap();
			if currency != "EUR" {
				return Err(Error::InvalidField(format!(
					"Unxpected currency {currency}"
				)));
			}
			let net_payable_eur_str = headers.get_field(&row, "Net Payable").unwrap();
			let net_payable_eur = net_payable_eur_str.parse::<BigDecimal>().unwrap();
			let net_payable_usd = net_payable_eur * source.eur_usd_rate.as_ref().unwrap();

			Ok(net_payable_usd.to_string())
		})
		.select(vec![
			"Reporting Period",
			"UPC",
			"ISRC",
			"Store",
			"Store service",
			"Units",
			"Net Payable",
			"Gross Royalties",
		])
}

fn landr(file_path: &PathBuf) -> Pipeline<'_> {
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

fn parse_pretzel_disbursement(text: &str) -> Result<NaiveDate, Error> {
	let mut text = text.to_string();
	if text.contains(" - Sup ") {
		// Ignore suplemental disbursements suffix, like "Mar 25 - Sup Apr 09 25"
		text = text.split(" - Sup ").collect::<Vec<_>>()[0].to_string();
	}
	let disbursement = match parse_date(&format!("1 {text}"), "%d %b %y") {
		Ok(date) => date,
		Err(e) => return Err(Error::InvalidField(e.to_string())),
	};
	Ok(disbursement)
}

fn pretzel(file_path: &PathBuf) -> Pipeline<'_> {
	Pipeline::from_path(file_path)
		.unwrap()
		.add_col("Reporting Period", |headers, row| {
			let disbursement_field = headers.get_field(&row, "disbursement").unwrap();
			let disbursement = parse_pretzel_disbursement(&disbursement_field)?;
			Ok(reporting_period_of(&disbursement))
		})
		.add_col("UPC", |headers, row| {
			// UPCs are not always available for reports up to 2020. Pretzel had
			// an old and new report system in use at once, and the old one lacks UPCs
			let disbursement_field = headers.get_field(&row, "disbursement").unwrap();
			let disbursement = parse_pretzel_disbursement(&disbursement_field)?;
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

fn read_repost_header(reader: csv::Reader<File>) -> csv::Reader<File> {
	let mut records = reader.into_records();

	let first = records.next().unwrap().unwrap();
	let has_account_id_prefix = first.len() == 1 && first.get(0) == Some("Account ID");
	let header = if has_account_id_prefix {
		records.next();
		records.next().unwrap().unwrap()
	} else {
		first
	};

	let mut reader = records.into_reader();
	reader.set_headers(header);
	reader
}

fn repost_network(file_path: &PathBuf) -> Pipeline<'_> {
	let file = match File::open(file_path) {
		Ok(file) => file,
		Err(_) => panic!("Could not open file: {}", file_path.to_string_lossy()),
	};

	let reader = csv::ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.from_reader(file);
	let reader = read_repost_header(reader);
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

fn stem(file_path: &PathBuf) -> Pipeline<'_> {
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

fn symphonic(file_path: &PathBuf) -> Pipeline<'_> {
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
		.filter_col("Digital Service Provider", |dsp| {
			dsp != "Balance Forwarded From Previous Quarter"
		})
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
		.map_col("UPC", |upc| match upc {
			"N/A" => Ok("".to_string()),
			_ => Ok(upc.to_string()),
		})
		.map_col("ISRC", |isrc| match isrc {
			"N/A" => Ok("".to_string()),
			_ => Ok(isrc.to_string()),
		})
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
