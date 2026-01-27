use crate::sources::{Source, SourceKind};
use bigdecimal::BigDecimal;
use deser_hjson;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
	pub is_initial: Option<bool>,
	#[serde(flatten)]
	pub sources_by_platform: HashMap<String, Vec<SourceManifest>>,
}
impl AccountingPeriodManifest {
	pub fn to_sources(&self, dir: &PathBuf) -> Vec<Source> {
		self.sources_by_platform
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
								source.file_path, self.name, platform
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
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum SourceManifest {
	Path(String),
	FullSource(SourceDetailsManifest),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SourceDetailsManifest {
	path: String,
	eur_usd_rate: Option<BigDecimal>,
	note: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum CatalogItem {
	Album(Album),
	Track(Track),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Album {
	pub upc: Option<String>,
	pub title: Option<String>,
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
