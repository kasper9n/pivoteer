use crate::settings::{Album, Settings, Track};
use crate::sources::Source;
use bigdecimal::{BigDecimal, FromPrimitive};
use csv_pipeline::{Pipeline, Transformer};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

pub struct Project {
	pub accounting_periods: Vec<AccountingPeriod>,
	/// We use a vec because multiple ISRCs can point to the same Track
	tracks: Vec<Track>,
	isrcs: HashMap<String, usize>,
	albums: HashMap<u64, Album>,
}
impl Project {
	fn new(dir: PathBuf, settings: Settings) -> Self {
		let accounting_periods = settings
			.accounting_periods
			.into_iter()
			.map(|accounting_period| {
				let sources = accounting_period.to_sources(&dir);
				AccountingPeriod {
					name: accounting_period.name,
					sources,
				}
			})
			.collect();
		let isrcs = {
			let mut isrc_map = HashMap::new();
			for (i, track) in settings.tracks.iter().enumerate() {
				for isrc in track.isrcs() {
					let old_vaue = isrc_map.insert(isrc.clone(), i);
					if old_vaue.is_some() {
						panic!("Duplicate ISRC found: {}", isrc);
					}
				}
			}
			isrc_map
		};
		let albums = {
			let mut album_map = HashMap::new();
			for album in settings.albums {
				if album.isrcs.is_empty() {
					panic!("Empty album {}", album.upc);
				}
				let old_vaue = album_map.insert(album.upc.clone(), album.clone());
				if old_vaue.is_some() {
					panic!("Duplicate UPC found: {}", album.upc);
				}
				for isrc in album.isrcs {
					if !isrcs.contains_key(&isrc) {
						panic!("Album {} contains non-existant ISRC {}", album.upc, isrc);
					}
				}
			}
			album_map
		};
		Project {
			accounting_periods,
			tracks: settings.tracks,
			isrcs,
			albums,
		}
	}
	pub fn load() -> Self {
		let settings_path = match env::args().nth(1) {
			Some(arg) => PathBuf::from(arg),
			None => {
				panic!("No Sources.toml argument given");
			}
		};
		let project_dir = settings_path.parent().unwrap().to_owned();
		let settings = Settings::from_path(settings_path);
		Self::new(project_dir, settings)
	}
	pub fn get_track(&self, isrc: &str) -> Option<&Track> {
		let index = *self.isrcs.get(isrc)?;
		Some(&self.tracks[index])
	}
}

pub struct AccountingPeriod {
	name: String,
	sources: Vec<Source>,
}
impl AccountingPeriod {
	fn generate_sales_report_csv_str(&self) -> String {
		let files: Vec<_> = self
			.sources
			.par_iter()
			.map(|source| {
				into_sales_report(source.process_source())
					.collect_into_rows()
					.unwrap()
			})
			.collect();
		let pipelines = files.into_iter().map(|rows| {
			return Pipeline::from_rows(rows).unwrap();
		});
		Pipeline::from_pipelines(pipelines)
			.collect_into_string()
			.unwrap()
	}
	pub fn generate_sales_report(&self) -> SalesReport {
		let sales_report_csv = self.generate_sales_report_csv_str();
		SalesReport::from_csv_str(sales_report_csv, self.name.clone())
	}
}

pub fn into_sales_report(pipeline: Pipeline) -> Pipeline {
	pipeline.transform_into(|| {
		vec![
			Transformer::new("Gross Royalties").sum(BigDecimal::from(0)),
			Transformer::new("ISRC").keep_unique(),
			Transformer::new("UPC").keep_unique(),
		]
	})
}

#[derive(Deserialize)]
struct SalesReportRecord {
	#[serde(rename = "Gross Royalties")]
	gross_royalties: BigDecimal,
	#[serde(rename = "ISRC")]
	isrc: String,
	#[serde(rename = "UPC")]
	upc: String,
}

pub struct SalesReport {
	isrc_map: HashMap<String, BigDecimal>,
	upc_map: HashMap<u64, BigDecimal>,
	accounting_period_name: String,
}
impl SalesReport {
	fn from_csv_str(sales_report_csv: String, accounting_period_name: String) -> Self {
		let mut rdr = csv::Reader::from_reader(sales_report_csv.as_bytes());

		let mut sales_report = Self {
			isrc_map: HashMap::new(),
			upc_map: HashMap::new(),
			accounting_period_name,
		};

		for result in rdr.deserialize() {
			let record: SalesReportRecord = result.unwrap();
			sales_report.add_sales_report_record(record);
		}
		sales_report
	}
	fn add_sales_report_record(&mut self, record: SalesReportRecord) {
		if record.isrc != "" {
			let entry = self
				.isrc_map
				.entry(record.isrc)
				.or_insert(record.gross_royalties.clone());
			*entry += record.gross_royalties;
		} else if record.upc != "" {
			let upc = record
				.upc
				.parse::<u64>()
				.expect(&format!("Invalid UPC {}", record.upc));
			let entry = self
				.upc_map
				.entry(upc)
				.or_insert(record.gross_royalties.clone());
			*entry += record.gross_royalties;
		} else {
			println!(
				"Missing UPC & ISRC in row with gross royalties of {}",
				record.gross_royalties
			);
		}
	}
	pub fn into_track_sales_report(self, project: &Project) -> TracksSalesReport {
		let mut isrc_report_map = self.isrc_map;

		for (upc, gross_royalty) in self.upc_map {
			let album = match project.albums.get(&upc) {
				Some(album) => album,
				None => {
					println!("No album with UPC {}", upc);
					continue;
				}
			};
			let album_len = BigDecimal::from_usize(album.isrcs.len()).unwrap();
			let sales_revenue_per_track = gross_royalty / album_len;
			for isrc in album.isrcs.clone() {
				*isrc_report_map.entry(isrc).or_default() += sales_revenue_per_track.clone()
			}
		}
		let tracks_map = isrc_report_map
			.into_iter()
			.map(|(isrc, gross_royalties)| {
				let track = match project.get_track(&isrc) {
					Some(val) => val,
					None => panic!("No track with ISRC {}", isrc),
				};
				let row = TrackSalesReportRow {
					isrc: isrc.clone(),
					title: track.title.clone(),
					gross_royalties,
				};
				(isrc, row)
			})
			.collect();
		TracksSalesReport {
			tracks: tracks_map,
			accounting_period_name: self.accounting_period_name,
		}
	}
}

#[derive(Debug)]
pub struct TrackSalesReportRow {
	pub isrc: String,
	pub title: String,
	pub gross_royalties: BigDecimal,
}
#[derive(Debug)]
pub struct TracksSalesReport {
	pub tracks: HashMap<String, TrackSalesReportRow>,
	pub accounting_period_name: String,
}
