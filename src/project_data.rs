use crate::earnings_report::{AccountingPeriod, Project};
use anyhow::{ensure, Context, Result};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectData {
	pub accounting_period_results: Vec<AccountingResult>,
}

impl ProjectData {
	pub fn open(data_file_path: &PathBuf) -> Result<Self> {
		let internal_data_str = fs::read_to_string(&data_file_path)?;
		Ok(serde_json::from_str(&internal_data_str).unwrap())
	}

	pub fn verify(&self, project: &Project) -> Result<()> {
		let accounting_periods = &project.accounting_periods;
		let result_periods = &self.accounting_period_results;
		for (sales_period, result) in accounting_periods.iter().zip(result_periods) {
			ensure!(
				sales_period.name == result.name,
				"Accounting period names do not match: \"{:?}\" != \"{:?}\"",
				sales_period.name,
				result.name
			);
		}
		if accounting_periods.len() < result_periods.len() {
			panic!("Accounting periods sales/result mismatch");
		}

		Ok(())
	}

	pub fn save(&self, data_file_path: &PathBuf) -> Result<()> {
		let file_string = serde_json::to_string_pretty(self).context("Failed to serialize data")?;
		fs::write(data_file_path, file_string).context("Failed to write data file")?;
		Ok(())
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountingResult {
	pub name: String,
	tracks: HashMap<String, TrackStatement>,
}

impl AccountingResult {
	pub fn generate(period: &AccountingPeriod, project: &Project) -> Result<AccountingResult> {
		let track_sales_report = period
			.generate_sales_report()
			.into_track_sales_report(&project);
		let track_recoupments = period.map_recoupments(project)?;

		let initial = AccountingResult {
			name: "Initial".to_string(),
			tracks: HashMap::new(),
		};

		let previous_result = match period.previous_period == "Initial" {
			true => &initial,
			false => project
				.data
				.accounting_period_results
				.iter()
				.find(|accounting_result| accounting_result.name == period.previous_period)
				.context(format!(
					"Could not find result from previous period \"{}\"",
					period.previous_period
				))?,
		};
		let previous_costs: HashMap<String, BigDecimal> = previous_result
			.tracks
			.iter()
			.map(|(isrc, track_statement)| (isrc.clone(), track_statement.net_amount.clone()))
			.collect();

		// Add previous costs
		let mut track_statements: HashMap<String, TrackStatement> = previous_costs
			.into_iter()
			.map(|(isrc, costs_from_previous)| {
				let mut statement = TrackStatement::default();
				statement.costs_from_previous = costs_from_previous;
				(isrc, statement)
			})
			.collect();

		// Add new costs
		for track_recoupment in track_recoupments.into_values() {
			let statement = track_statements.entry(track_recoupment.isrc).or_default();
			statement.new_costs = track_recoupment.recoup;
		}

		// Add track statements for everything in the sales report
		for (isrc, sales_info) in track_sales_report.tracks {
			let statement = track_statements.entry(isrc).or_default();
			let track = project.get_track(&sales_info.isrc).unwrap();

			let gross_royalties = sales_info.gross_royalties;
			let costs_from_previous = statement.costs_from_previous.clone();
			let new_costs = statement.new_costs.clone();
			let total_costs = costs_from_previous.clone() - new_costs.clone();
			let net_amount = gross_royalties.clone() - total_costs;
			let payable = net_amount.clone().max(BigDecimal::from(0));

			*statement = TrackStatement {
				isrc: sales_info.isrc,
				title: sales_info.title,
				gross_royalties,
				costs_from_previous,
				new_costs,
				net_amount,
				splits: track
					.splits
					.iter()
					.map(|split| TrackStatementSplits {
						share: split.share.clone(),
						name: split.name.clone(),
						net_royalties: payable.clone() * split.share.clone(),
					})
					.collect(),
			};
		}

		Ok(Self {
			name: period.name.clone(),
			tracks: track_statements,
		})
	}
}

/// ## Example
///
/// | Type            |   Jan |   Feb |   Apr |   Mar |   May |
/// | --------------- | ----- | ----- | ----- | ----- | ----- |
/// | gross_royalties |    50 |    50 |   250 |   200 |   150 |
/// | b/f costs       |     0 |  -250 |  -200 |     0 |     0 |
/// | new costs       |  -300 |     0 |     0 |     0 |  -200 |
/// | net_sales       |  -250 |  -200 |    50 |   200 |   -50 |
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackStatement {
	isrc: String,
	title: String,
	gross_royalties: BigDecimal,
	/// future_recoupment from the previous accounting period
	costs_from_previous: BigDecimal,
	/// new recoupable costs
	new_costs: BigDecimal,
	/// Actual amount recouped
	net_amount: BigDecimal,
	splits: Vec<TrackStatementSplits>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackStatementSplits {
	pub share: BigDecimal,
	pub name: String,
	pub net_royalties: BigDecimal,
}
