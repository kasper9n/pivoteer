use anyhow::{bail, ensure, Result};
use bigdecimal::{BigDecimal, Signed};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::project::YearQuarter;

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
	pub inernal_data_file: String,
	pub accounting_periods: Vec<AccountingPeriodManifest>,
	pub catalog: Vec<CatalogItem>,
}
impl Manifest {
	pub fn from_path(file_path: PathBuf) -> Self {
		let file_str = fs::read_to_string(&file_path).unwrap();

		{
			// discard the value, because we would lose error position information
			let value: serde_json::Value = json5::from_str(&file_str).unwrap();
			validate_manifest_json(&value, "");
		}

		let json_deserializer = &mut json5::Deserializer::from_str(&file_str);
		let result: Result<Manifest, _> = serde_path_to_error::deserialize(json_deserializer);

		let manifest = match result {
			Ok(m) => m,
			Err(err) => {
				let inner = err.inner();
				let snippet = extract_by_path(&file_str, err.path());
				panic!(
					"Error at {}:\n  {inner}\n\nOffending JSON:{}\n",
					err.path().to_string(),
					snippet
						.map(|s| format!("\n{s}"))
						.unwrap_or_else(|| " (could not extract)".into())
				)
			}
		};
		manifest
	}
	pub fn verify(&self) -> Result<()> {
		for (i, accounting_period) in self.accounting_periods.iter().enumerate() {
			match accounting_period.is_initial {
				Some(true) => ensure!(i == 0, "First accounting period must have is_initial"),
				None => ensure!(i > 0, "Only first accounting period must have is_initial"),
				Some(false) => bail!("is_initial cannot be false"),
			}
		}
		Ok(())
	}
	pub fn all_tracks(&self) -> Vec<Track> {
		self.catalog
			.iter()
			.flat_map(|item| match item {
				CatalogItem::Track(track) => vec![track.clone()],

				CatalogItem::Album(album) => album
					.tracks
					.iter()
					.filter_map(|album_track| match album_track {
						AlbumTrack::Track(t) => Some(t.clone()),
						AlbumTrack::Isrc(_) => None,
					})
					.collect(),
			})
			.collect()
	}
}

fn extract_by_path(json: &str, path: &serde_path_to_error::Path) -> Option<String> {
	use serde_path_to_error::Segment;

	let v: serde_json::Value = json5::from_str(json).unwrap();
	let mut current = &v;

	for segment in path {
		match segment {
			Segment::Map { key } => {
				current = current.get(key).unwrap();
			}
			Segment::Seq { index } => {
				current = current.get(*index).unwrap();
			}
			Segment::Enum { .. } => {
				// Enum variant — current should already be the variant's data
			}
			Segment::Unknown => {
				// Can't navigate further
				return None;
			}
		}
	}

	serde_json::to_string_pretty(current).ok()
}

fn validate_manifest_json(value: &serde_json::Value, path: &str) {
	match value {
		serde_json::Value::Number(_) => {
			// https://github.com/akubera/bigdecimal-rs/issues/113
			panic!("Number values are not allowed because they cause precision loss. Path: {path}");
		}
		serde_json::Value::Object(map) => {
			for (key, value) in map {
				let path = format!("{path}.{key}");
				let path = path.trim_start_matches('.');
				RecoupmentManifest::prohibit_partial_json(map, &*path);
				validate_manifest_json(value, path);
			}
		}
		serde_json::Value::Array(array) => {
			for (i, value) in array.iter().enumerate() {
				validate_manifest_json(value, &format!("{path}[{i}]"));
			}
		}
		serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::String(_) => {}
	}
}

#[derive(Serialize, Deserialize, Debug)]
// deny_unknown_fields not supported with flatten
pub struct AccountingPeriodManifest {
	pub name: YearQuarter,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_initial: Option<bool>,
	#[serde(flatten)]
	pub sources_by_platform: BTreeMap<String, Vec<SourceManifest>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum SourceManifest {
	Path(String),
	FullSource(SourceDetailsManifest),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct SourceDetailsManifest {
	pub path: String,
	pub eur_usd_rate: Option<BigDecimal>,
	note: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum CatalogItem {
	Album(AlbumManifest),
	Track(Track),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// deny_unknown_fields not supported with flatten
pub struct AlbumManifest {
	pub upc: String,
	pub title: String,
	pub tracks: Vec<AlbumTrack>,
	#[serde(flatten)]
	pub recoupment: Option<RecoupmentManifest>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum AlbumTrack {
	Isrc(String),
	Track(Track),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// deny_unknown_fields not supported with flatten
pub struct Track {
	#[serde(rename = "isrc")]
	pub main_isrc: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secondary_isrcs: Option<Vec<String>>,
	pub single_upcs: Vec<String>,
	pub title: String,
	pub label_share: BigDecimal,
	pub splits: Vec<Split>,
	#[serde(flatten)]
	pub recoupment: Option<RecoupmentManifest>,
}
impl Track {
	pub fn isrcs(&self) -> Vec<String> {
		let mut isrcs = vec![self.main_isrc.clone()];
		if let Some(secondary_isrcs) = &self.secondary_isrcs {
			isrcs.extend(secondary_isrcs.clone());
		}
		isrcs
	}
	// pub fn id(&self) -> String {
	// 	// 32-bit, alphanumeric without 0OIL
	// 	// 123456789abcdefghjkmnpqrstuvwxyz
	// }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// deny_unknown_fields not supported with flatten
pub struct RecoupmentManifest {
	pub max_recoup: BigDecimal,
	pub expenses: BigDecimal,
	pub recoup: BigDecimal,
	pub recoupments: Vec<RecoupableCost>,
}
impl RecoupmentManifest {
	pub fn prohibit_partial_json(map: &serde_json::Map<String, serde_json::Value>, path: &str) {
		if map.contains_key("recoup") && map.contains_key("date") {
			// allow RecoupableCost
			return;
		}
		let recoup_keys = ["max_recoup", "expenses", "recoup", "recoupments"];
		let mut count = 0;
		for key in map.keys() {
			if recoup_keys.contains(&key.as_str()) {
				count += 1;
			}
		}
		if count != 0 && count != recoup_keys.len() {
			panic!("Invalid (partial) recoupments fields. Path: {path}");
		}
	}
	pub fn validate(&self) -> Result<()> {
		let mut total_recoup = BigDecimal::from(0);
		let mut total_expenses = BigDecimal::from(0);
		for recoupment in &self.recoupments {
			total_recoup += &recoupment.recoup;
			total_expenses += &recoupment.expense;
			ensure!(total_recoup >= 0);
			ensure!(total_expenses >= 0);
			ensure!(
				recoupment.recoup <= recoupment.expense,
				"Recouped more than the expense: {:?}",
				recoupment
			);
			ensure!(
				total_recoup <= self.max_recoup,
				"Track recoupment exceeds max_recoup: {:?}",
				recoupment
			);
			ensure!(
				recoupment.note.is_some()
					|| !(recoupment.expense.is_negative() && !recoupment.recoup.is_negative()),
				"Negative recoupment must have a note: {:?}",
				recoupment
			);
		}
		ensure!(
			total_expenses == self.expenses,
			"Expenses sum does {total_expenses} not match listed expenses {}",
			self.expenses,
		);
		ensure!(
			total_recoup == self.recoup,
			"Recoup sum {total_recoup} does not match listed recoup {}",
			self.recoup,
		);
		Ok(())
	}
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct RecoupableCost {
	pub date: String,
	pub expense: BigDecimal,
	pub recoup: BigDecimal,
	pub note: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Split {
	pub share: BigDecimal,
	pub share_composition: Option<BigDecimal>,
	pub name: String,
}
