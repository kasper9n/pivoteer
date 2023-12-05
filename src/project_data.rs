use crate::earnings_report::{AccountingPeriod, Project};
use crate::settings::Payout;
use crate::to_json_string_pretty;
use crate::track_sales_report::TrackSalesReport;
use anyhow::{ensure, Context, Result};
use bigdecimal::{BigDecimal, Signed, Zero};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn pct_to_factor(share: &BigDecimal) -> BigDecimal {
	share * BigDecimal::from_str("0.01").unwrap()
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
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

	pub fn get_accounting_result(&self, name: &str) -> Option<&AccountingResult> {
		self.accounting_period_results
			.iter()
			.find(|accounting_period| accounting_period.name == name)
	}

	pub fn save(&self, data_file_path: &PathBuf) -> Result<()> {
		let buf = to_json_string_pretty(&self)?;
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccountingResult {
	pub name: String,
	pub previous_name: Option<String>,
	pub is_initial: bool,
	payouts: Vec<Payout>,
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
		let artist_statements =
			Self::generate_artist_statements(&track_statements, period, &project)?;
		Ok(AccountingResult {
			name: period.name.clone(),
			previous_name: period.previous_period.clone(),
			is_initial: period.is_initial,
			payouts: period.payouts.clone(),
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

		let previous_net_amounts: HashMap<String, BigDecimal> = match &period.previous_period {
			Some(previous_period) => {
				let previous_result = project
					.data
					.accounting_period_results
					.iter()
					.find(|accounting_result| accounting_result.name == *previous_period)
					.context(format!(
						"Could not find result from previous period \"{}\"",
						previous_period
					))?;
				previous_result
					.track_statements
					.iter()
					.map(|(isrc, track_statement)| {
						(isrc.clone(), track_statement.net_amount.clone())
					})
					.collect()
			}
			None => HashMap::new(),
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

			*statement = TrackStatement {
				isrc: "".to_string(),
				title: "".to_string(),
				opening_net_amount,
				gross_royalties,
				new_costs,
				net_amount: BigDecimal::zero(),
			};
		}

		// Fill in remaining fields for all elements
		for (isrc, statement) in &mut statements {
			let track = project.get_track(&isrc).unwrap();
			statement.isrc = track.main_isrc.clone();
			statement.net_amount = statement.opening_net_amount.clone()
				+ statement.gross_royalties.clone()
				- statement.new_costs.clone();
			statement.title = track.title.clone();

			ensure!(isrc != "");
			ensure!(isrc == statement.isrc.as_str());
		}

		Ok(statements)
	}
	fn generate_artist_statements(
		track_statements: &TrackStatementsMap,
		period: &AccountingPeriod,
		project: &Project,
	) -> Result<ArtistStatementsMap> {
		let mut artist_statements: ArtistStatementsMap = HashMap::new();
		let previous_result = match &period.previous_period {
			None => None,
			Some(previous_period) => {
				Some(project.data.get_accounting_result(previous_period).unwrap())
			}
		};

		for track_statement in track_statements.values() {
			let track = project.get_track(&track_statement.isrc).unwrap();
			for split in &track.splits {
				let artist_statement =
					artist_statements
						.entry(split.name.clone())
						.or_insert_with(|| {
							return ArtistStatement {
								net_royalties: BigDecimal::zero(),
								tracks: vec![],
							};
						});
				let artist_track_statement = ArtistTrackStatement {
					isrc: track_statement.isrc.clone(),
					net_royalties: track_statement.payable() * pct_to_factor(&split.share),
				};
				if artist_track_statement.net_royalties.is_positive() {
					artist_statement.net_royalties += artist_track_statement.net_royalties.clone();
				}
				artist_statement.tracks.push(artist_track_statement);
				// sort tracks for determinism
				artist_statement.tracks.sort_unstable_by(|a, b| {
					// net royalties (descending)
					b.net_royalties
						.cmp(&a.net_royalties)
						.then_with(|| a.isrc.cmp(&b.isrc))
						.then_with(|| panic!("Duplicate ISRCs in artist statement"))
				})
			}
		}

		// Add statements for artists that have past statements, but no track royalties this period
		if let Some(previous_result) = previous_result {
			for (name, _statement) in previous_result.artist_statements.iter() {
				artist_statements
					.entry(name.clone())
					.or_insert(ArtistStatement {
						net_royalties: BigDecimal::zero(),
						tracks: vec![],
					});
			}
		}

		// Add statements for artists that were paid (advance), but have no track royalties this period
		for payout in period.payouts.iter() {
			let name = payout.name.clone();
			artist_statements.entry(name).or_insert_with(|| {
				println!(
					"Warning: Payout to artist that has no track royalties \"{}\"",
					payout.name
				);
				ArtistStatement {
					net_royalties: BigDecimal::zero(),
					tracks: vec![],
				}
			});
		}

		Ok(artist_statements)
	}
	pub fn export(&self) -> Export {
		let mut artist_statements: Vec<_> = self
			.artist_statements
			.iter()
			.map(|(payee, statement)| {
				let mut tracks: Vec<_> = statement
					.tracks
					.iter()
					.map(|ats| {
						let track_statement = self.track_statements.get(&ats.isrc).unwrap();
						let payable = track_statement.payable();
						return ArtistTrackStatementExport {
							isrc: ats.isrc.clone(),
							title: track_statement.title.clone(),
							gross_royalties: track_statement.gross_royalties.clone(),
							payable_royalties: payable.clone(),
							net_royalties: ats.net_royalties.clone(),
						};
					})
					.collect();
				tracks.sort_by(|a, b| {
					b.net_royalties
						.cmp(&a.net_royalties)
						.then_with(|| a.isrc.cmp(&b.isrc))
						.then_with(|| panic!("Sort duplicate artist statement track"))
				});
				return ArtistStatementExport {
					payee: payee.clone(),
					net_royalties: statement.net_royalties.to_string(),
					tracks,
				};
			})
			.collect();
		artist_statements.sort_by(|a, b| {
			numeric_sort::cmp(&b.net_royalties, &a.net_royalties)
				.then_with(|| numeric_sort::cmp(&b.payee, &a.payee))
				.then_with(|| panic!("Sort duplicate artist statement"))
		});
		Export {
			name: self.name.clone(),
			previous_name: self.previous_name.clone().into(),
			is_initial: self.is_initial,
			artist_statements,
		}
	}
}

#[derive(Serialize)]
pub struct Export {
	name: String,
	previous_name: Option<String>,
	is_initial: bool,
	artist_statements: Vec<ArtistStatementExport>,
}
impl Export {
	pub fn save_to_downloads<P: AsRef<Path>>(&self, file_name: P) -> Result<()> {
		let buf = to_json_string_pretty(&self)?;
		let file_path = dirs_next::download_dir()
			.context("Failed to get download dir")?
			.join(file_name);
		let mut file = OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&file_path)
			.with_context(|| format!("Could not export to {:?}", file_path))?;
		file.write_all(&buf)?;
		Ok(())
	}
}
#[derive(Serialize)]
pub struct ArtistStatementExport {
	payee: String,
	net_royalties: String,
	tracks: Vec<ArtistTrackStatementExport>,
}
#[derive(Serialize)]
struct ArtistTrackStatementExport {
	pub isrc: String,
	pub title: String,
	pub gross_royalties: BigDecimal,
	/// Artist-payable royalties from this statement. Gross royalties - costs - negative opening balance
	pub payable_royalties: BigDecimal,
	pub net_royalties: BigDecimal,
}

#[cfg(test)]
mod test {
	use crate::earnings_report::Project;
	use crate::project_data::ProjectData;
	use anyhow::Result;
	use pretty_assertions::assert_eq;
	use std::path::PathBuf;

	#[test]
	fn test_generate_artist_statements() -> Result<()> {
		let mut project = Project::load(PathBuf::from("test/Settings.jsonc"))?;

		let q1_result = project
			.get_accounting_period("1999 Q1")
			.unwrap()
			.generate_result(&project)?;
		project.add_result(q1_result)?;

		let q2_result = project
			.get_accounting_period("1999 Q2")
			.unwrap()
			.generate_result(&project)?;
		project.add_result(q2_result)?;

		let expected_data =
			ProjectData::open(&PathBuf::from("test/Internal data expected.json")).unwrap();

		assert_eq!(
			project.data.accounting_period_results,
			expected_data.accounting_period_results
		);
		Ok(())
	}
}

/// ISRC -> TrackStatement
type TrackStatementsMap = HashMap<String, TrackStatement>;

/// ## Example
///
/// | Type               |   Jan |   Feb |   Apr |   Mar |   May |
/// | ------------------ | ----- | ----- | ----- | ----- | ----- |
/// | opening_net_amount |     0 |  -250 |  -200 |    50 |   250 |
/// | gross_royalties    |    50 |    50 |   250 |   200 |    50 |
/// | new_costs          |  -300 |     0 |     0 |     0 |  -400 |
/// | new_amount         |  -250 |  -200 |    50 |   250 |  -100 |
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
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
impl TrackStatement {
	/// Artist-payable royalties from this statement. Gross royalties - costs - negative opening balance
	pub fn payable(&self) -> BigDecimal {
		let negative_opening_balance = self.opening_net_amount.clone().min(BigDecimal::zero());
		self.gross_royalties.clone() - self.new_costs.clone() + negative_opening_balance
	}
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ArtistStatement {
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
