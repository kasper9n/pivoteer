use anyhow::{ensure, Result};
use bigdecimal::BigDecimal;
use earnings_report::Project;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

mod earnings_report;
mod settings;
mod sources;
mod transformers;

fn main() -> Result<()> {
	let project = Project::load()?;
	project.verify()?;
	let track_sales_report = project
		.accounting_periods
		.par_iter()
		.map(|accounting_period| {
			let sales_report = accounting_period.generate_sales_report();
			let track_sales_report = sales_report.into_track_sales_report(&project);
			track_sales_report
		})
		.collect::<Vec<_>>();

	let internal_data = InternalData::load(&project)?;

	// store internal file with all generated data
	// generate track sales report for the first accounting period
	// accounting periods have a "previous_period" field for good measure
	// check previous_period
	//   - if first: use 0 values
	//   - if exists: use calculated values
	Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InternalData {
	accounting_period_results: Vec<AccountingPeriodData>,
}

impl InternalData {
	fn load(project: &Project) -> Result<Self> {
		let internal_data_str = fs::read_to_string(&project.data_file_path)?;
		let internal_data: Self = serde_json::from_str(&internal_data_str).unwrap();

		let sales_periods = &project.accounting_periods;
		let result_periods = &internal_data.accounting_period_results;
		for (sales_period, result) in sales_periods.iter().zip(result_periods) {
			ensure!(
				sales_period.name == result.name,
				"Accounting period names do not match: \"{:?}\" != \"{:?}\"",
				sales_period.name,
				result.name
			);
		}
		if sales_periods.len() < result_periods.len() {
			panic!("Accounting periods sales/result mismatch");
		}

		Ok(internal_data)
	}
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountingPeriodData {
	name: String,
	tracks: HashMap<String, TrackData>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackData {
	isrc: String,
	title: String,
	gross_royalties: BigDecimal,
	/// future_recoupment from the previous accounting period
	recoupment_from_previous: BigDecimal,
	/// new recoupable costs
	new_recoupment: BigDecimal,
	/// Actual amount recouped
	recouped: BigDecimal,
	/// Recoupment to bring forward to next accounting period
	future_recoupment: BigDecimal,
	artists: Vec<TrackArtistData>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackArtistData {
	pub share: BigDecimal,
	pub name: String,
	pub net_royalties: BigDecimal,
}
