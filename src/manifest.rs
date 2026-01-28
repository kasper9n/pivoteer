use anyhow::{ensure, Result};
use bigdecimal::{BigDecimal, Signed};
use deser_hjson;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

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

		let value: serde_json::Value = deser_hjson::from_str(&file_str).unwrap();
		prohibit_number_values(&value);

		deser_hjson::from_str(&file_str).unwrap()
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

fn prohibit_number_values(value: &serde_json::Value) {
	match value {
		serde_json::Value::Number(_) => {
			// https://github.com/akubera/bigdecimal-rs/issues/113
			panic!("Number values are not allowed because they cause precision loss")
		}
		serde_json::Value::Object(map) => {
			for (_, value) in map {
				prohibit_number_values(value);
			}
		}
		serde_json::Value::Array(array) => {
			for value in array {
				prohibit_number_values(value);
			}
		}
		serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::String(_) => {}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountingPeriodManifest {
	pub name: String,
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Track {
	#[serde(rename = "isrc")]
	pub main_isrc: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secondary_isrcs: Option<Vec<String>>,
	pub single_upcs: Vec<String>,
	pub title: String,
	pub label_share: BigDecimal,
	pub splits: Vec<Split>,
	pub max_recoup: BigDecimal,
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
pub struct RecoupmentManifest {
	pub expenses: BigDecimal,
	pub recoup: BigDecimal,
	pub recoupments: Vec<RecoupableCost>,
}
impl RecoupmentManifest {
	pub fn verify(&self, max_recoup: &BigDecimal) -> Result<()> {
		let mut total_recoup = BigDecimal::from(0);
		let mut total_expenses = BigDecimal::from(0);
		for recoupment in &self.recoupments {
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
				&total_recoup <= max_recoup,
				"Track recoupment exceeds max_recoup: {:?}",
				recoupment
			);
			ensure!(
				recoupment.note.is_some()
					|| (recoupment.expense.is_positive() && recoupment.recoup.is_positive()),
				"Negative recoupment must have a note: {:?}",
				recoupment
			);
		}
		ensure!(
			total_expenses == self.expenses,
			"Expenses sum does not match listed expenses",
		);
		ensure!(
			total_recoup == self.recoup,
			"Recoup sum does not match listed recoup",
		);
		Ok(())
	}
}

#[derive(Clone, Serialize, Deserialize, Debug)]
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
