use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::sources::{Source, SourceKind};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
	pub accounting_periods: Vec<AccountingPeriodInfo>,
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
