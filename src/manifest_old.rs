use crate::manifest::{
	self, AccountingPeriodManifest, AlbumManifest, AlbumTrack, CatalogItem, Manifest,
	RecoupableCost, RecoupmentManifest, SourceManifest, Split,
};
use bigdecimal::BigDecimal;
use deser_hjson;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
	pub inernal_data_file: String,
	pub accounting_periods: Vec<AccountingPeriodInfo>,
	pub tracks: Vec<Track>,
	pub albums: Vec<Album>,
}
impl Settings {
	pub fn from_path(file_path: PathBuf) -> Self {
		let file_str = fs::read_to_string(&file_path).unwrap();

		let value: serde_json::Value = deser_hjson::from_str(&file_str).unwrap();
		prohibit_number_values(&value);

		deser_hjson::from_str(&file_str).unwrap()
	}
	pub fn migrate(self) -> Manifest {
		let mut new_accounting_periods = Vec::new();
		let mut recoupments = Vec::new();
		for accounting_period in self.accounting_periods {
			let new_accounting_period = AccountingPeriodManifest {
				name: accounting_period.name,
				is_initial: accounting_period.is_initial,
				sources_by_platform: accounting_period.sources_by_platform,
			};
			new_accounting_periods.push(new_accounting_period);
			for recoupment in accounting_period.recoupments {
				recoupments.push(recoupment);
			}
		}

		let mut catalog = Vec::new();

		for album in self.albums {
			let new_album = AlbumManifest {
				upc: album.upc,
				title: album.title,
				tracks: album
					.isrcs
					.into_iter()
					.map(|isrc| AlbumTrack::Isrc(isrc))
					.collect(),
				recoupment: None,
			};
			catalog.push(CatalogItem::Album(new_album));
		}

		for track in self.tracks {
			let mut track_recoupments = Vec::new();
			recoupments.retain(|recoupment| {
				if recoupment.isrc == track.main_isrc {
					track_recoupments.push(recoupment.clone());
					false
				} else {
					true
				}
			});
			let new_track = manifest::Track {
				main_isrc: track.main_isrc,
				secondary_isrcs: track.secondary_isrcs,
				single_upcs: track.single_upcs,
				title: track.title,
				label_share: track.label_share,
				splits: track.splits,
				max_recoup: track.max_recoup,
				recoupment: Some(RecoupmentManifest {
					expenses: track.expenses,
					recoup: track.recoup,
					recoupments: track_recoupments
						.into_iter()
						.map(|cost| RecoupableCost {
							expense: cost.expense.clone(),
							recoup: cost.recoup.clone(),
							date: cost.date.clone(),
							note: cost.note.clone(),
						})
						.collect(),
				}),
			};
			catalog.push(CatalogItem::Track(new_track));
		}
		assert!(recoupments.is_empty());

		Manifest {
			inernal_data_file: self.inernal_data_file,
			accounting_periods: new_accounting_periods,
			catalog,
		}
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

#[derive(Serialize, Deserialize)]
pub struct AccountingPeriodInfo {
	pub name: String,
	pub previous_period: Option<String>,
	pub is_initial: Option<bool>,
	pub recoupments: Vec<Recoupment>,
	#[serde(flatten)]
	pub sources_by_platform: BTreeMap<String, Vec<SourceManifest>>,
}

// #[derive(Serialize, Deserialize)]
// #[serde(untagged)]
// pub enum AccountingPeriodInfoSource {
// 	Path(String),
// 	FullSource(SourceInfo),
// }

// #[derive(Serialize, Deserialize)]
// pub struct SourceInfo {
// 	path: String,
// 	eur_usd_rate: Option<BigDecimal>,
// 	note: Option<String>,
// }

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Recoupment {
	pub isrc: String,
	pub date: String,
	pub expense: BigDecimal,
	pub recoup: BigDecimal,
	pub name: String,
	pub note: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Album {
	pub upc: String,
	pub title: String,
	pub isrcs: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
	#[serde(rename = "isrc")]
	pub main_isrc: String,
	pub secondary_isrcs: Option<Vec<String>>,
	pub single_upcs: Vec<String>,
	pub title: String,
	pub max_recoup: BigDecimal,
	pub expenses: BigDecimal,
	pub recoup: BigDecimal,
	pub label_share: BigDecimal,
	pub splits: Vec<Split>,
}

impl Track {
	pub fn isrcs(&self) -> Vec<String> {
		let mut isrcs = vec![self.main_isrc.clone()];
		if let Some(secondary_isrcs) = &self.secondary_isrcs {
			isrcs.extend(secondary_isrcs.clone());
		}
		isrcs
	}
}
