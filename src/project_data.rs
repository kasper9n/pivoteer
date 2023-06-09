use crate::earnings_report::{AccountingPeriod, Project};
use crate::track_sales_report::TrackSalesReport;
use anyhow::{ensure, Context, Result};
use bigdecimal::{BigDecimal, Zero};
use serde::{Deserialize, Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

fn pct_to_factor(share: &BigDecimal) -> BigDecimal {
	share * BigDecimal::from_str("0.01").unwrap()
}

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
		let mut buf = Vec::new();
		let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
		let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
		self.serialize(&mut ser)
			.context("Failed to serialize data")?;
		fs::write(data_file_path, buf).context("Failed to write data file")?;
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
	track_statements: TrackStatementsMap,
	#[serde(serialize_with = "sorted_map")]
	artist_statements: HashMap<String, ArtistStatement>,
}

impl AccountingResult {
	pub fn generate(period: &AccountingPeriod, project: &Project) -> Result<AccountingResult> {
		let sales_report = period.generate_sales_report();
		let track_sales_report = sales_report.into_track_sales_report(&project);
		let track_statements =
			Self::generate_track_statements(track_sales_report, period, project)?;
		let artist_statements = Self::generate_artist_statements(&track_statements, &project)?;
		Ok(AccountingResult {
			name: period.name.clone(),
			track_statements,
			artist_statements,
		})
	}
	fn generate_track_statements(
		track_sales_report: TrackSalesReport,
		period: &AccountingPeriod,
		project: &Project,
	) -> Result<TrackStatementsMap> {
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
		let mut statements: TrackStatementsMap = previous_net_amounts
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
			};
		}

		// Fill in remaining fields for all elements
		for (isrc, statement) in &mut statements {
			let track = project.get_track(&isrc).unwrap();
			statement.isrc = track.main_isrc.clone();
			statement.title = track.title.clone();

			ensure!(isrc != "");
			ensure!(isrc == statement.isrc.as_str());
		}

		Ok(statements)
	}
	fn generate_artist_statements(
		track_statements: &TrackStatementsMap,
		project: &Project,
	) -> Result<ArtistStatementsMap> {
		let mut artist_statements: ArtistStatementsMap = HashMap::new();

		for track_statement in track_statements.values() {
			let track = project.get_track(&track_statement.isrc).unwrap();
			for split in &track.splits {
				let artist_statement =
					artist_statements
						.entry(split.name.clone())
						.or_insert_with(|| ArtistStatement {
							name: split.name.clone(),
							net_royalties: BigDecimal::zero(),
							tracks: vec![],
						});
				let artist_track_statement = ArtistTrackStatement {
					isrc: track_statement.isrc.clone(),
					net_royalties: track_statement.net_amount.clone() * pct_to_factor(&split.share),
				};
				artist_statement.net_royalties += artist_track_statement
					.net_royalties
					.clone()
					.max(BigDecimal::zero());
				artist_statement.tracks.push(artist_track_statement);
				// sort tracks for determinism
				artist_statement.tracks.sort_unstable_by(|a, b| {
					// net royalties (descending)
					match b.net_royalties.cmp(&a.net_royalties) {
						// fallback to isrcs (ascending)
						Ordering::Equal => match a.isrc.cmp(&b.isrc) {
							Ordering::Equal => {
								panic!("Duplicate ISRCs in artist statement")
							}
							order => order,
						},
						order => order,
					}
				})
			}
		}

		Ok(artist_statements)
	}
}

#[cfg(test)]
mod test {
	use crate::project_data::{AccountingResult, ArtistTrackStatement, TrackStatement};
	use bigdecimal::BigDecimal;
	use maplit::hashmap;

	#[test]
	fn test_generate_artist_statements() {
		let project = crate::earnings_report::test::create_mock_project();
		let track_statements = hashmap! {
			"A".to_string() => TrackStatement {
				isrc: "A".to_string(),
				title: "Salvatore Ganacci - Fight Dirty".to_string(),
				net_amount: BigDecimal::from(20),
				..Default::default()
			},
			"B".to_string() => TrackStatement {
				isrc: "B".to_string(),
				title: "Salvatore Ganacci - Take Me To America".to_string(),
				net_amount: BigDecimal::from(-6),
				..Default::default()
			},
		};
		let artist_statements =
			AccountingResult::generate_artist_statements(&track_statements, &project).unwrap();
		assert_eq!(artist_statements.len(), 1);
		assert_eq!(
			artist_statements["Salvatore Ganacci"].net_royalties,
			BigDecimal::from(10)
		);
		assert_eq!(
			artist_statements["Salvatore Ganacci"].tracks,
			vec![
				ArtistTrackStatement {
					isrc: "A".to_string(),
					net_royalties: BigDecimal::from(10),
				},
				ArtistTrackStatement {
					isrc: "B".to_string(),
					net_royalties: BigDecimal::from(0),
				},
			]
		);
	}
}

type TrackStatementsMap = HashMap<String, TrackStatement>;

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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtistStatement {
	name: String,
	net_royalties: BigDecimal,
	tracks: Vec<ArtistTrackStatement>,
}
type ArtistStatementsMap = HashMap<String, ArtistStatement>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ArtistTrackStatement {
	pub isrc: String,
	pub net_royalties: BigDecimal,
}
