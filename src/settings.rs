use crate::sources::{Source, SourceKind};
use bigdecimal::BigDecimal;
use deser_hjson;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

		let value: serde_json::Value = serde_json::from_str(&file_str).unwrap();
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

#[derive(Serialize, Deserialize)]
pub struct AccountingPeriodInfo {
	pub name: String,
	pub previous_period: Option<String>,
	pub is_initial: Option<bool>,
	pub recoupments: Vec<Recoupment>,
	#[serde(flatten)]
	pub sources_by_platform: HashMap<String, Vec<String>>,
}
impl AccountingPeriodInfo {
	pub fn to_sources(&self, dir: &PathBuf) -> Vec<Source> {
		self.sources_by_platform
			.iter()
			.map(|(platform, file_paths)| {
				let kind = SourceKind::from_str(platform);
				let sources = file_paths
					.into_iter()
					.map(|file_path| {
						let file_path = PathBuf::from(dir).join(file_path);
						if !Path::exists(&file_path) {
							panic!(
								"File not found: {:?}. From {} {}",
								file_path, self.name, platform
							);
						}
						Source { file_path, kind }
					})
					.collect::<Vec<_>>();
				sources
			})
			.flatten()
			.collect()
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Recoupment {
	pub isrc: String,
	pub date: String,
	pub expense: BigDecimal,
	pub recoup: BigDecimal,
	pub name: String,
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

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Split {
	pub share: BigDecimal,
	pub name: String,
}
