use crate::project_data::{AccountingResult, ProjectData};
use crate::settings::{Album, RecoupmentSetup, Setup, Track};
use crate::sources::Source;
use crate::track_sales_report::TrackSalesReport;
use anyhow::{bail, ensure, Context, Result};
use bigdecimal::{BigDecimal, Signed};
use csv_pipeline::{Pipeline, Transformer};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
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
	fn new(dir: PathBuf, setup: Setup) -> Self {
		let accounting_periods = setup
			.accounting_periods
			.into_iter()
			.map(|accounting_period| {
				let sources = accounting_period.to_sources(&dir);
				AccountingPeriod {
					name: accounting_period.name,
					is_initial: accounting_period.is_initial.unwrap_or(false),
					sources,
				}
			})
			.collect();
		let mut album_map = HashMap::new();
		let mut track_map = HashMap::new();
		for (i, track) in setup.tracks.iter().enumerate() {
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
		for album in setup.albums {
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
		let data_file_path = dir.join(setup.inernal_data_file);
		let data = ProjectData::open(&data_file_path)
			.context("Failed to open internal data file")
			.unwrap();
		Project {
			data_file_path,
			accounting_periods,
			data,
			tracks: setup.tracks,
			isrcs: track_map,
			albums: album_map,
		}
	}
	pub fn load(settings_path: PathBuf) -> Result<Self> {
		let project_dir = settings_path.parent().unwrap().to_owned();
		let settings = Setup::from_path(settings_path);
		Ok(Self::new(project_dir, settings))
	}
	pub fn verify(&self) -> Result<()> {
		if !Path::exists(&self.data_file_path) {
			bail!("Internal data file not found: {:?}", self.data_file_path);
		}
		self.verify_accounting_periods()?;
		self.verify_tracks()?;
		self.verify_sources()?;
		self.data.verify(self)?;
		Ok(())
	}
	fn verify_sources(&self) -> Result<()> {
		let mut paths = HashSet::new();
		for accounting_period in &self.accounting_periods {
			for source in &accounting_period.sources {
				let is_new = paths.insert(source.file_path.canonicalize()?);
				if !is_new {
					bail!(
						"Source file is listed multiple times: {:?}",
						source.file_path
					);
				}
			}
		}
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
			let is_full_label_share = summed_artist_shares == BigDecimal::from(0)
				&& track.label_share == BigDecimal::from(100);
			if summed_artist_shares != BigDecimal::from(100) && !is_full_label_share {
				bail!(
					"Track \"{}\" splits don't add up: {:#?}",
					track.title,
					track.splits
				);
			}

			let recoupment_setup = match &track.recoupment {
				Some(track_recoupment) => track_recoupment,
				None => continue,
			};
			let mut total_recoup = BigDecimal::from(0);
			let mut total_expenses = BigDecimal::from(0);
			for recoupment in &recoupment_setup.recoupments {
				total_recoup += &recoupment.recoup;
				total_expenses += &recoupment.expense;
				ensure!(total_recoup > 0);
				ensure!(total_expenses > 0);
				ensure!(
					recoupment.recoup <= recoupment.expense,
					"Recouped more than the expense: {:?}",
					recoupment
				);
				ensure!(
					recoupment.recoup <= track.max_recoup,
					"Track recoupment exceeds max_group: {}",
					track.title,
				);
				ensure!(
					recoupment.note.is_some()
						|| (recoupment.expense.is_positive() && recoupment.recoup.is_positive()),
					"Negative recoupment must have a note: {:?}",
					recoupment
				);
			}
		}
		Ok(())
	}
	fn verify_accounting_periods(&self) -> Result<()> {
		if self.accounting_periods.is_empty() {
			bail!("No accounting periods found")
		}

		let mut accounting_periods_iter = self.accounting_periods.iter().enumerate().peekable();
		while let Some((i, accounting_period)) = accounting_periods_iter.next() {
			if i == 0 {
				ensure!(
					accounting_period.is_initial == true,
					"First accounting period must have is_initial set to true"
				);
			}
			if let Some((_, next_accounting_period)) = accounting_periods_iter.peek() {
				ensure!(
					next_accounting_period.prev_period() == accounting_period.name.clone(),
					"Accounting period \"{}\" has unexpected previous period \"{:?}\"",
					accounting_period.name,
					next_accounting_period.prev_period(),
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
	pub fn add_result(&mut self, result: AccountingResult) -> Result<()> {
		let period = self.get_accounting_period(&result.name).unwrap();
		match self.data.accounting_period_results.last() {
			Some(last_result) => {
				if last_result.name.clone() != period.prev_period() {
					bail!("Last result not previous_period. Maybe this was already generated?");
				}
			}
			None => {
				ensure!(
					period.is_initial == true,
					"No results exist, yet the accounting period does not have is_initial set to true"
				);
			}
		}
		self.data.accounting_period_results.push(result);
		match self.data.verify(&self) {
			Ok(_) => {}
			Err(e) => {
				self.data.accounting_period_results.pop();
				bail!(e);
			}
		};
		Ok(())
	}
	pub fn add_and_save_result(&mut self, result: AccountingResult) -> Result<()> {
		self.add_result(result)?;
		self.data.save(&self.data_file_path)?;
		Ok(())
	}
}

#[derive(Clone)]
pub struct AccountingPeriod {
	pub name: String,
	pub is_initial: bool,
	sources: Vec<Source>,
}
impl AccountingPeriod {
	pub fn year(&self) -> u16 {
		YearQuarter::parse(&self.name).year
	}
	pub fn quarter(&self) -> u8 {
		YearQuarter::parse(&self.name).quarter
	}
	pub fn prev_period(&self) -> String {
		let current = YearQuarter::parse(&self.name);
		let prev = current.get_prev();
		format!("{} Q{}", prev.year, prev.quarter)
	}
	fn generate_sales_report_csv_str(&self) -> String {
		let files: Vec<_> = self
			.sources
			.par_iter()
			.map(|source| {
				into_sales_report(source.process_source())
					.collect_into_rows()
					.map_err(|e| panic!("Error processing source {:?}: {:?}", source.file_path, e))
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
	pub fn recoupments_by_track(
		&self,
		project: &Project,
	) -> Result<HashMap<String, RecoupmentSetup>> {
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
					.or_insert(RecoupmentSetup {
						isrc: recoupment.isrc.clone(),
						upc: recoupment.upc.clone(),
						date: recoupment.date.clone(),
						expense: BigDecimal::from(0),
						recoup: BigDecimal::from(0),
						name: recoupment.name.clone(),
						note: None,
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

#[derive(Clone, Debug)]
pub struct YearQuarter {
	year: u16,
	quarter: u8,
}
impl YearQuarter {
	pub fn parse(s: &str) -> Self {
		let parts = s.split(" ").collect::<Vec<_>>();
		assert_eq!(parts.len(), 2);

		let value = Self {
			year: parts[0].parse().unwrap(),
			quarter: parts[1].parse().unwrap(),
		};
		value.assert_valid();

		value
	}
	pub fn assert_valid(&self) {
		assert!((1000..=9999).contains(&self.year));
		assert!((1..=4).contains(&self.quarter));
	}
	pub fn get_prev(&self) -> Self {
		let mut value = self.clone();
		if value.quarter == 1 {
			value.quarter = 4;
			value.year -= 1;
		} else {
			value.quarter -= 1;
		}
		value.assert_valid();
		value
	}
}

pub fn into_sales_report(pipeline: Pipeline) -> Pipeline {
	pipeline
		.map(|headers, row| {
			let isrc = headers.get_field(&row, "ISRC").unwrap();
			let upc = headers.get_field(&row, "UPC").unwrap();
			if isrc == "" && upc == "" {
				println!("Missing ISRC & UPC: {:?}", row);
			}
			Ok(row)
		})
		.transform_into(|| {
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
				"Missing UPC & ISRC in row with gross royalties of {}. Row UPC \"{}\", ISRC \"{}\"",
				record.gross_royalties, record.upc, record.isrc
			);
		}
	}
	pub fn into_track_sales_report(self, project: &Project) -> TrackSalesReport {
		TrackSalesReport::from_sales_report(self, project)
	}
}
