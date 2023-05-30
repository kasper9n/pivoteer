use crate::settings::{Album, Recoupment, Settings, Track};
use crate::sources::Source;
use anyhow::{bail, ensure, Result};
use bigdecimal::{BigDecimal, FromPrimitive};
use csv_pipeline::{Pipeline, Transformer};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

pub struct Project {
	pub data_file_path: PathBuf,
	pub accounting_periods: Vec<AccountingPeriod>,
	pub recoupments: Vec<Recoupment>,
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
					previous_period: accounting_period.previous_period,
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
		let data_file_path = dir.join(settings.inernal_data_file);
		if !Path::exists(&data_file_path) {
			panic!("Internal data file not found: {:?}", data_file_path);
		}
		Project {
			data_file_path,
			recoupments: settings.recoupments,
			accounting_periods,
			tracks: settings.tracks,
			isrcs,
			albums,
		}
	}
	pub fn load() -> Result<Self> {
		let settings_path = match env::args().nth(1) {
			Some(arg) => PathBuf::from(arg),
			None => bail!("No Sources.toml argument given"),
		};
		let project_dir = settings_path.parent().unwrap().to_owned();
		let settings = Settings::from_path(settings_path);
		Ok(Self::new(project_dir, settings))
	}
	pub fn verify(&self) -> Result<()> {
		if !Path::exists(&self.data_file_path) {
			bail!("Internal data file not found: {:?}", self.data_file_path);
		}
		self.verify_accounting_periods()?;
		self.verify_recoupments()?;
		Ok(())
	}
	fn verify_accounting_periods(&self) -> Result<()> {
		if self.accounting_periods.is_empty() {
			bail!("No accounting periods found")
		}

		let mut accounting_periods_iter = self.accounting_periods.iter().peekable();
		while let Some(accounting_period) = accounting_periods_iter.next() {
			ensure!(
				accounting_period.name != "Initial",
				"Accounting periods cannot be named \"Initial\"",
			);
			if let Some(next_accounting_period) = accounting_periods_iter.peek() {
				ensure!(
					next_accounting_period.previous_period == accounting_period.name,
					"Accounting period \"{}\" has previous_period \"{}\", but previous is named \"{}\"",
					next_accounting_period.name,
					next_accounting_period.previous_period,
					accounting_period.name,
				)
			}
		}
		Ok(())
	}
	fn verify_recoupments(&self) -> Result<()> {
		let mut track_recoupments = HashMap::new();
		for recoupment in &self.recoupments {
			let track = match self.get_track_by_main_isrc(&recoupment.isrc) {
				Some(track) => track,
				None => bail!("Recoupment has a non-existent ISRC {}", recoupment.isrc),
			};
			ensure!(
				recoupment.name == track.title,
				"Recoupment track title mismatch:\n{}\n{}",
				recoupment.name,
				track.title,
			);
			let track_recoupment =
				track_recoupments
					.entry(track.main_isrc.clone())
					.or_insert(Recoupment {
						isrc: recoupment.isrc.clone(),
						date: recoupment.date.clone(),
						expense: BigDecimal::from(0),
						recoup: BigDecimal::from(0),
						name: recoupment.name.clone(),
					});
			track_recoupment.expense += recoupment.expense.clone();
			track_recoupment.recoup += recoupment.recoup.clone();
			ensure!(
				track_recoupment.recoup <= track.max_recoup,
				"Track recoupment exceeds max_group: {}",
				track_recoupment.name,
			);
			ensure!(
				track_recoupment.recoup <= track_recoupment.expense,
				"{} track recoupment exceeds expenses: {}",
				track_recoupment.date,
				track_recoupment.name,
			);
		}

		for track_recoupment in track_recoupments.values() {
			let track = self.get_track_by_main_isrc(&track_recoupment.isrc).unwrap();
			ensure!(
				track_recoupment.expense == track.expenses,
				"Track recoupment expense mismatch: {}",
				track_recoupment.name
			);
			ensure!(
				track_recoupment.recoup == track.recoup,
				"Track recoup mismatch: {}",
				track_recoupment.name
			);
		}

		for track in &self.tracks {
			let track_recoupment = track_recoupments.get(&track.main_isrc);
			let track_recoupment_expense = track_recoupment
				.map(|r| r.expense.clone())
				.unwrap_or(BigDecimal::from(0));
			let track_recoupment_recoup = track_recoupment
				.map(|r| r.recoup.clone())
				.unwrap_or(BigDecimal::from(0));
			ensure!(
				track_recoupment_expense == track.expenses,
				"Recoupment expense mismatch for {}:\n{} != {}",
				track.title,
				track_recoupment_expense,
				track.expenses,
			);
			ensure!(
				track_recoupment_recoup == track.recoup,
				"Track recoup mismatch: {}",
				track.title,
			);
		}

		Ok(())
	}
	pub fn get_track(&self, isrc: &str) -> Option<&Track> {
		let index = *self.isrcs.get(isrc)?;
		Some(&self.tracks[index])
	}
	pub fn get_track_by_main_isrc(&self, isrc: &str) -> Option<&Track> {
		let track = self.get_track(isrc)?;
		match track.main_isrc == isrc {
			true => Some(track),
			false => None,
		}
	}
}

pub struct AccountingPeriod {
	pub name: String,
	pub previous_period: String,
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
	pub fn into_track_sales_report(self, project: &Project) -> TrackSalesReport {
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
		TrackSalesReport {
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
pub struct TrackSalesReport {
	pub tracks: HashMap<String, TrackSalesReportRow>,
	pub accounting_period_name: String,
}
