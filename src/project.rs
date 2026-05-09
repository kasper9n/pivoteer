use crate::manifest::{
	AlbumManifest, AlbumTrack, CatalogItem, Manifest, RecoupmentManifest, Track,
};
use crate::project_data::{AccountingData, AccountingPeriodResult};
use crate::sources::{parse_date, Source};
use crate::track_sales_report::TrackSalesReport;
use anyhow::{bail, ensure, Context, Result};
use bigdecimal::BigDecimal;
use csv_pipeline::{Pipeline, Transformer};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct Project {
	pub data_file_path: PathBuf,
	pub accounting_periods: Vec<AccountingPeriod>,
	pub data: AccountingData,
	/// We use a vec because multiple ISRCs can point to the same Track
	pub tracks: Vec<Track>,
	isrcs: HashMap<String, usize>,
	pub albums: HashMap<String, Album>,
}
impl Project {
	pub fn load(manifest_path: PathBuf) -> Result<Self> {
		let project_dir = manifest_path.parent().unwrap().to_owned();
		let manifest = Manifest::from_path(manifest_path);
		manifest.verify()?;
		let accounting_periods = manifest
			.accounting_periods
			.iter()
			.map(|accounting_period| AccountingPeriod {
				name: accounting_period.name.clone(),
				is_initial: accounting_period.is_initial.unwrap_or(false),
				sources: Source::from_manifest(&accounting_period, &project_dir),
			})
			.collect();

		let all_tracks = manifest.all_tracks();
		let mut album_map = HashMap::new();
		let mut track_map: HashMap<String, usize> = HashMap::new();

		for (i, track) in all_tracks.clone().into_iter().enumerate() {
			// Insert track ISRCs into track_map
			for isrc in track.isrcs() {
				match track_map.entry(isrc.clone()) {
					Entry::Occupied(_) => bail!("Duplicate ISRC found: {isrc}"),
					Entry::Vacant(entry) => entry.insert(i),
				};
			}
			// Insert track singles into album_map (single_upc)
			for single_upc in &track.single_upcs {
				match album_map.entry(single_upc.clone()) {
					Entry::Occupied(_) => bail!("Duplicate UPC found: {single_upc}"),
					Entry::Vacant(entry) => entry.insert(Album {
						upc: single_upc.clone(),
						title: track.title.clone(),
						isrcs: vec![track.main_isrc.clone()],
						// We don't copy the recoupment because it already exists in the track
						recoupment: None,
					}),
				};
			}
		}

		// Add albums to album_map
		for catalog_item in manifest.catalog {
			let album = match catalog_item {
				CatalogItem::Album(album) => Album::from_manifest(album),
				CatalogItem::Track(_) => continue,
			};
			let upc = album.upc.clone();
			match album_map.entry(album.upc.clone()) {
				Entry::Occupied(_) => bail!("Duplicate UPC found: {upc}"),
				Entry::Vacant(entry) => entry.insert(album),
			};
		}

		let data_file_path = project_dir.join(manifest.inernal_data_file);
		let data = AccountingData::open(&data_file_path)
			.context("Failed to open internal data file")
			.unwrap();

		Ok(Project {
			data_file_path,
			accounting_periods,
			data,
			tracks: all_tracks,
			isrcs: track_map,
			albums: album_map,
		})
	}
	pub fn validate(&self) -> Result<()> {
		if !Path::exists(&self.data_file_path) {
			bail!("Internal data file not found: {:?}", self.data_file_path);
		}
		self.validate_accounting_periods()?;
		self.validate_tracks()?;
		self.validate_albums()?;
		self.validate_sources()?;
		self.data.validate(self)?;
		Ok(())
	}
	fn validate_sources(&self) -> Result<()> {
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
	fn validate_tracks(&self) -> Result<()> {
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

			if let Some(track_recoupment) = &track.recoupment {
				track_recoupment
					.validate()
					.context(format!("Invalid recoupment for {}", track.title))?;
			}
		}
		Ok(())
	}
	fn validate_albums(&self) -> Result<()> {
		for (upc, album) in &self.albums {
			ensure!(!album.isrcs.is_empty(), "Empty album {upc}");

			let mut tracks = Vec::new();
			for isrc in &album.isrcs {
				// Allow secondary ISRCs to be used in albums
				match self.get_track_by_any_isrc(isrc) {
					Some(track) => tracks.push(track),
					None => bail!("Album {upc} contains non-existant ISRC {isrc}"),
				}
			}

			for track in tracks {
				if album.recoupment.is_some() {
					ensure!(
						track.recoupment.is_none(),
						"Recoupment cannot be on both the track and it's album. Track {}",
						track.title
					);
				}
			}

			if let Some(album_recoupment) = &album.recoupment {
				album_recoupment.validate()?;
			}
		}
		Ok(())
	}
	fn validate_accounting_periods(&self) -> Result<()> {
		if self.accounting_periods.is_empty() {
			bail!("No accounting periods found")
		}

		let mut accounting_periods_iter = self.accounting_periods.iter().enumerate().peekable();
		while let Some((i, accounting_period)) = accounting_periods_iter.next() {
			match accounting_period.is_initial {
				true => ensure!(i == 0, "First accounting period not is_initial"),
				false => ensure!(i != 0, "Non-first accounting period with is_initial"),
			}
			if let Some((_, next_accounting_period)) = accounting_periods_iter.peek() {
				ensure!(
					next_accounting_period.prev_period() == accounting_period.name.clone(),
					"Accounting period \"{}\" has unexpected previous period \"{:?}\"",
					accounting_period.name.to_string(),
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
	pub fn get_track(&self, isrc: &str) -> Result<&Track> {
		let track = self
			.get_track_by_any_isrc(isrc)
			.context(format!("Track with ISRC {isrc} not found"))?;
		match track.main_isrc == isrc {
			true => Ok(track),
			false => bail!("Track ISRC {isrc} is a secondary ISRC"),
		}
	}
	pub fn get_album(&self, upc: &str) -> Option<&Album> {
		self.albums.get(upc)
	}
	pub fn get_album_containing_isrc(&self, isrc: &str) -> Option<&Album> {
		for (_, album) in &self.albums {
			if album.isrcs.contains(&isrc.to_string()) {
				return Some(album);
			}
		}
		None
	}
	pub fn get_accounting_period(&self, name: &YearQuarter) -> Option<&AccountingPeriod> {
		self.accounting_periods
			.iter()
			.find(|accounting_period| &accounting_period.name == name)
	}
	pub fn add_result(&mut self, result: AccountingPeriodResult) -> Result<()> {
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
		match self.data.validate(&self) {
			Ok(_) => {}
			Err(e) => {
				self.data.accounting_period_results.pop();
				bail!(e);
			}
		};
		Ok(())
	}
	pub fn add_and_save_result(&mut self, result: AccountingPeriodResult) -> Result<()> {
		self.add_result(result)?;
		self.data.save(&self.data_file_path)?;
		Ok(())
	}
}

#[derive(Clone)]
pub struct Album {
	pub upc: String,
	#[allow(unused)]
	pub title: String,
	pub isrcs: Vec<String>,
	pub recoupment: Option<RecoupmentManifest>,
}
impl Album {
	pub fn from_manifest(album: AlbumManifest) -> Self {
		let main_isrcs = album
			.tracks
			.iter()
			.map(|track| match track {
				AlbumTrack::Isrc(isrc) => isrc.clone(),
				AlbumTrack::Track(track) => track.main_isrc.clone(),
			})
			.collect();
		Album {
			upc: album.upc.clone(),
			title: album.title.clone(),
			isrcs: main_isrcs,
			recoupment: album.recoupment,
		}
	}
}

#[derive(Clone)]
pub struct AccountingPeriod {
	pub name: YearQuarter,
	pub is_initial: bool,
	sources: Vec<Source>,
}
impl AccountingPeriod {
	pub fn prev_period(&self) -> YearQuarter {
		self.name.get_prev()
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct YearQuarter {
	year: u16,
	quarter: u8,
}
impl YearQuarter {
	pub fn parse(s: &str) -> Self {
		let parts = s.split(" Q").collect::<Vec<_>>();
		assert_eq!(parts.len(), 2);

		let value = Self {
			year: parts[0].parse().unwrap(),
			quarter: parts[1].parse().unwrap(),
		};
		value.validate();

		value
	}
	pub fn validate(&self) {
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
		value.validate();
		value
	}
	pub fn contains_date(&self, date: &str) -> bool {
		let date = parse_date(date, "%Y-%m-%d").unwrap();
		let period_of_date = date.format("%Y %q").to_string();
		period_of_date == format!("{} {}", self.year, self.quarter)
	}
	// pub fn end_date(&self) -> NaiveDate {
	// 	match self.quarter {
	// 		1 => NaiveDate::from_ymd_opt(self.year.into(), 3, 31).unwrap(),
	// 		2 => NaiveDate::from_ymd_opt(self.year.into(), 6, 30).unwrap(),
	// 		3 => NaiveDate::from_ymd_opt(self.year.into(), 9, 30).unwrap(),
	// 		4 => NaiveDate::from_ymd_opt(self.year.into(), 12, 31).unwrap(),
	// 		_ => panic!("Invalid quarter"),
	// 	}
	// }
}
impl ToString for YearQuarter {
	fn to_string(&self) -> String {
		format!("{} Q{}", self.year, self.quarter)
	}
}
impl Serialize for YearQuarter {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.validate();
		serializer.serialize_str(&self.to_string())
	}
}
impl<'de> Deserialize<'de> for YearQuarter {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let value = Self::parse(&s);
		Ok(value)
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
	pub accounting_period_name: YearQuarter,
	pub isrc_map: HashMap<String, BigDecimal>,
	pub upc_map: HashMap<String, BigDecimal>,
}
impl SalesReport {
	fn from_csv_str(sales_report_csv: String, accounting_period_name: YearQuarter) -> Self {
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
