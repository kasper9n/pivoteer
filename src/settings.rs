use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::sources::{Source, SourceKind};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
	pub accounting_periods: Vec<AccountingPeriodInfo>,
	pub tracks: Vec<Track>,
	pub albums: Vec<Album>,
}
impl Settings {
	pub fn from_path(file_path: PathBuf) -> Self {
		let settings_str = fs::read_to_string(&file_path).unwrap();
		serde_json::from_str(&settings_str).unwrap()
	}
}

#[derive(Serialize, Deserialize)]
pub struct AccountingPeriodInfo {
	pub name: String,
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
						Source {
							file_path: PathBuf::from(dir).join(file_path),
							kind,
						}
					})
					.collect::<Vec<_>>();
				sources
			})
			.flatten()
			.collect()
	}
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Album {
	pub upc: u64,
	pub title: String,
	pub isrcs: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
	#[serde(rename = "isrc")]
	pub main_isrc: String,
	pub secondary_isrcs: Option<Vec<String>>,
	pub single_upc: Option<u64>,
	pub title: String,
	pub max_recoup: BigDecimal,
	pub expenses: BigDecimal,
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Split {
	pub share: BigDecimal,
	pub name: String,
}
