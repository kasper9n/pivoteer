use crate::earnings_report::{AccountingPeriod, Project};
use anyhow::{ensure, Context, Result};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
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

// https://stackoverflow.com/a/74971717/6553404
pub fn sorted_map<S: Serializer, K: Serialize + Ord, V: Serialize>(
	value: &HashMap<K, V>,
	serializer: S,
) -> Result<S::Ok, S::Error> {
	let mut items: Vec<(_, _)> = value.iter().collect();
	items.sort_by(|a, b| a.0.cmp(&b.0));
	BTreeMap::from_iter(items).serialize(serializer)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountingResult {
	pub name: String,
	#[serde(serialize_with = "sorted_map")]
	track_statements: TrackStatements,
	#[serde(serialize_with = "sorted_map")]
	artist_statements: HashMap<String, ArtistStatement>,
}

impl AccountingResult {
	pub fn generate(period: &AccountingPeriod, project: &Project) -> Result<AccountingResult> {
		let track_statements = Self::generate_track_statements(period, project)?;
		let artist_statements = Self::generate_artist_statements(&track_statements)?;
		Ok(AccountingResult {
			name: period.name.clone(),
			track_statements,
			artist_statements,
		})
	}
	fn generate_track_statements(
		period: &AccountingPeriod,
		project: &Project,
	) -> Result<TrackStatements> {
		let track_sales_report = period
			.generate_sales_report()
			.into_track_sales_report(&project);
		let track_recoupments = period.map_recoupments(project)?;

		let previous_net_amounts: HashMap<String, BigDecimal> = if period.previous_period
			== "Initial"
		{
			HashMap::new()
		} else {
			let previous_result = project
				.data
				.accounting_period_results
				.iter()
				.find(|accounting_result| accounting_result.name == period.previous_period)
				.context(format!(
					"Could not find result from previous period \"{}\"",
					period.previous_period
				))?;
			previous_result
				.track_statements
				.iter()
				.map(|(isrc, track_statement)| (isrc.clone(), track_statement.net_amount.clone()))
				.collect()
		};

		// Add previous costs
		let mut statements: TrackStatements = previous_net_amounts
			.into_iter()
			.map(|(isrc, previous_net_amounts)| {
				let mut statement = TrackStatement::default();
				statement.opening_net_amount = previous_net_amounts;
				statement.isrc = isrc.clone();
				(isrc, statement)
			})
			.collect();

		// Add new costs
		for track_recoupment in track_recoupments.into_values() {
			let statement = statements
				.entry(track_recoupment.isrc.clone())
				.or_insert_with(|| TrackStatement {
					isrc: track_recoupment.isrc,
					..Default::default()
				});
			statement.new_costs = track_recoupment.recoup;
		}

		// Add track statements for everything in the sales report
		for (isrc, sales_info) in track_sales_report.tracks {
			let statement = statements.entry(isrc).or_default();

			let opening_net_amount = statement.opening_net_amount.clone();
			let gross_royalties = sales_info.gross_royalties;
			let new_costs = statement.new_costs.clone();
			let net_amount =
				opening_net_amount.clone() + gross_royalties.clone() - new_costs.clone();

			*statement = TrackStatement {
				isrc: "".to_string(),
				title: "".to_string(),
				opening_net_amount,
				gross_royalties,
				new_costs,
				net_amount: net_amount.clone(),
				splits: vec![],
			};
		}

		// Fill in remaining fields for all elements
		for (isrc, statement) in &mut statements {
			let track = project.get_track(&isrc).unwrap();
			statement.isrc = track.main_isrc.clone();
			statement.title = track.title.clone();
			statement.splits = track
				.splits
				.iter()
				.map(|split| TrackStatementSplits {
					share: split.share.clone(),
					name: split.name.clone(),
					net_royalties: statement.net_amount.clone() * (split.share.clone() / 100),
				})
				.collect();

			ensure!(isrc != "");
			ensure!(isrc == statement.isrc.as_str());
		}

		Ok(statements)
	}
	fn generate_artist_statements(track_statements: &TrackStatements) -> Result<ArtistStatements> {
		let mut artist_statements: ArtistStatements = HashMap::new();

		for track_statement in track_statements.values() {
			for split in &track_statement.splits {
				artist_statements
					.entry(split.name.clone())
					.and_modify(|artist_statement| {
						artist_statement.net_royalties +=
							split.net_royalties.clone().max(BigDecimal::from(0));
					})
					.or_insert_with(|| ArtistStatement {
						name: split.name.clone(),
						net_royalties: split.net_royalties.clone(),
					});
			}
		}

		Ok(artist_statements)
	}
}

type TrackStatements = HashMap<String, TrackStatement>;

/// ## Example
///
/// | Type               |   Jan |   Feb |   Apr |   Mar |   May |
/// | ------------------ | ----- | ----- | ----- | ----- | ----- |
/// | opening_net_amount |     0 |  -250 |  -200 |    50 |   250 |
/// | gross_royalties    |    50 |    50 |   250 |   200 |    50 |
/// | new_costs          |  -300 |     0 |     0 |     0 |  -400 |
/// | new_amount         |  -250 |  -200 |    50 |   250 |  -100 |
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackStatement {
	isrc: String,
	title: String,
	/// All-time track royalties minus all-time track costs
	opening_net_amount: BigDecimal,
	gross_royalties: BigDecimal,
	/// New recoupable costs
	new_costs: BigDecimal,
	/// All-time track royalties minus all-time track costs
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtistStatement {
	name: String,
	net_royalties: BigDecimal,
}
type ArtistStatements = HashMap<String, ArtistStatement>;
