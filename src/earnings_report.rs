use crate::project_data::{AccountingResult, ProjectData};
use crate::settings::{Album, Recoupment, Settings, Track};
use crate::sources::Source;
use crate::track_sales_report::TrackSalesReport;
use anyhow::{bail, ensure, Result};
use bigdecimal::BigDecimal;
use csv_pipeline::{Pipeline, Transformer};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct Project {
	pub data_file_path: PathBuf,
	pub accounting_periods: Vec<AccountingPeriod>,
	pub data: ProjectData,
	/// We use a vec because multiple ISRCs can point to the same Track
	tracks: Vec<Track>,
	isrcs: HashMap<String, usize>,
	albums: HashMap<String, Album>,
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
					recoupments: accounting_period.recoupments,
				}
			})
			.collect();
		let mut album_map = HashMap::new();
		let mut track_map = HashMap::new();
		for (i, track) in settings.tracks.iter().enumerate() {
			for isrc in track.isrcs() {
				let replaced_track = track_map.insert(isrc.clone(), i);
				if replaced_track.is_some() {
					panic!("Duplicate ISRC found: {}", isrc);
				}
			}
			for single_upc in &track.single_upcs {
				let replaced_album = album_map.insert(
					single_upc.clone(),
					Album {
						isrcs: vec![track.main_isrc.clone()],
						upc: single_upc.clone(),
						title: track.title.clone(),
					},
				);
				if replaced_album.is_some() {
					panic!("Duplicate UPC found: {}", single_upc);
				}
			}
		}
		for album in settings.albums {
			if album.isrcs.is_empty() {
				panic!("Empty album {}", album.upc);
			}
			let replaced_album = album_map.insert(album.upc.clone(), album.clone());
			if replaced_album.is_some() {
				panic!("Duplicate UPC found: {}", album.upc);
			}
			for isrc in album.isrcs {
				if !track_map.contains_key(&isrc) {
					panic!("Album {} contains non-existant ISRC {}", album.upc, isrc);
				}
			}
		}
		let data_file_path = dir.join(settings.inernal_data_file);
		let data = ProjectData::open(&data_file_path).unwrap();
		Project {
			data_file_path,
			accounting_periods,
			data,
			tracks: settings.tracks,
			isrcs: track_map,
			albums: album_map,
		}
	}
	pub fn load(settings_path: PathBuf) -> Result<Self> {
		let project_dir = settings_path.parent().unwrap().to_owned();
		let settings = Settings::from_path(settings_path);
		Ok(Self::new(project_dir, settings))
	}
	pub fn verify(&self) -> Result<()> {
		if !Path::exists(&self.data_file_path) {
			bail!("Internal data file not found: {:?}", self.data_file_path);
		}
		self.verify_accounting_periods()?;
		self.verify_tracks()?;
		self.data.verify(self)?;
		Ok(())
	}
	fn verify_tracks(&self) -> Result<()> {
		for track in &self.tracks {
			let summed_artist_shares = track
				.splits
				.iter()
				.map(|split| split.share.clone())
				.reduce(|acc, share| acc + share)
				.unwrap_or(BigDecimal::from(0));
			let summed_shares = track.label_share.clone() + summed_artist_shares;
			if summed_shares != BigDecimal::from(100) {
				bail!("Track {} splits don't add up", track.title);
			}
		}
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
	pub fn get_track_by_any_isrc(&self, isrc: &str) -> Option<&Track> {
		let index = *self.isrcs.get(isrc)?;
		Some(&self.tracks[index])
	}
	pub fn get_track(&self, isrc: &str) -> Option<&Track> {
		let track = self.get_track_by_any_isrc(isrc)?;
		match track.main_isrc == isrc {
			true => Some(track),
			false => None,
		}
	}
	pub fn get_album(&self, upc: &str) -> Option<&Album> {
		self.albums.get(upc)
	}
	pub fn get_accounting_period(&self, name: &str) -> Option<&AccountingPeriod> {
		self.accounting_periods
			.iter()
			.find(|accounting_period| accounting_period.name == name)
	}
	pub fn save_result(&mut self, result: AccountingResult) -> Result<()> {
		let period = self.get_accounting_period(&result.name).unwrap();
		match self.data.accounting_period_results.last() {
			Some(last_result) => {
				if last_result.name != period.previous_period {
					bail!("Last result not previous_period");
				}
			}
			None => {
				if period.previous_period != "Initial" {
					panic!("No results exist, yet previous_period != Initial");
				}
			}
		}
		self.data.accounting_period_results.push(result);
		self.data.verify(&self)?;
		self.data.save(&self.data_file_path)?;
		Ok(())
	}
}

pub struct AccountingPeriod {
	pub name: String,
	pub previous_period: String,
	pub recoupments: Vec<Recoupment>,
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
	pub fn generate_result(&self, project: &Project) -> Result<AccountingResult> {
		AccountingResult::generate(self, project)
	}
	pub fn map_recoupments(&self, project: &Project) -> Result<HashMap<String, Recoupment>> {
		let mut track_recoupments = HashMap::new();
		for recoupment in &self.recoupments {
			let track = match project.get_track(&recoupment.isrc) {
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
		Ok(track_recoupments)
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

#[derive(Debug, Deserialize)]
struct SalesReportRecord {
	// String because deserializing to BigDecimal seems to lose precision
	#[serde(rename = "Gross Royalties")]
	gross_royalties: String,
	#[serde(rename = "ISRC")]
	isrc: String,
	#[serde(rename = "UPC")]
	upc: String,
}

#[derive(Debug)]
pub struct SalesReport {
	pub isrc_map: HashMap<String, BigDecimal>,
	pub upc_map: HashMap<String, BigDecimal>,
	pub accounting_period_name: String,
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
				.or_insert(BigDecimal::from(0));
			*entry += BigDecimal::from_str(&record.gross_royalties).unwrap();
		} else if record.upc != "" {
			let entry = self
				.upc_map
				.entry(record.upc)
				.or_insert(BigDecimal::from(0));
			*entry += BigDecimal::from_str(&record.gross_royalties).unwrap();
		} else {
			println!(
				"Missing UPC & ISRC in row with gross royalties of {}",
				record.gross_royalties
			);
		}
	}
	pub fn into_track_sales_report(self, project: &Project) -> TrackSalesReport {
		TrackSalesReport::from_sales_report(self, project)
	}
}

#[cfg(test)]
pub mod test {
	use crate::earnings_report::Project;
	use crate::project_data::ProjectData;
	use crate::settings::{Split, Track};
	use bigdecimal::{BigDecimal, Zero};
	use maplit::hashmap;
	use std::path::PathBuf;
	pub fn create_mock_project() -> Project {
		Project {
			data_file_path: PathBuf::new(),
			accounting_periods: vec![],
			data: ProjectData {
				accounting_period_results: vec![],
			},
			tracks: vec![
				Track {
					main_isrc: "A".to_string(),
					title: "Salvatore Ganacci - Fight Dirty".to_string(),
					secondary_isrcs: None,
					single_upcs: vec![],
					max_recoup: BigDecimal::zero(),
					expenses: BigDecimal::zero(),
					recoup: BigDecimal::zero(),
					label_share: BigDecimal::zero(),
					splits: vec![Split {
						name: "Salvatore Ganacci".to_string(),
						share: BigDecimal::from(50),
					}],
				},
				Track {
					main_isrc: "B".to_string(),
					title: "Salvatore Ganacci - Take Me To America".to_string(),
					secondary_isrcs: None,
					single_upcs: vec![],
					max_recoup: BigDecimal::zero(),
					expenses: BigDecimal::zero(),
					recoup: BigDecimal::zero(),
					label_share: BigDecimal::zero(),
					splits: vec![Split {
						name: "Salvatore Ganacci".to_string(),
						share: BigDecimal::from(50),
					}],
				},
			],
			isrcs: hashmap! {
				"A".to_string() => 0,
				"B".to_string() => 1,
			},
			albums: hashmap! {},
		}
	}
}
