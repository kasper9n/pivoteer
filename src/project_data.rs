use crate::accounting::{AccountId, Voucher};
use crate::project::{Project, YearQuarter};
use crate::to_json_string_pretty;
use anyhow::{bail, ensure, Context, Result};
use bigdecimal::{BigDecimal, Zero};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::value::RawValue;
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn pct_to_factor(share: &BigDecimal) -> BigDecimal {
	share * BigDecimal::from_str("0.01").unwrap()
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct AccountingData {
	pub accounting_period_results: Vec<AccountingPeriodResult>,
}
impl AccountingData {
	pub fn open(data_file_path: &PathBuf) -> Result<Self> {
		let internal_data_str = fs::read_to_string(&data_file_path)?;
		Ok(serde_json::from_str(&internal_data_str).unwrap())
	}

	pub fn validate(&self, project: &Project) -> Result<()> {
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

		for (i, accounting_period) in self.accounting_period_results.iter().enumerate() {
			match accounting_period.is_initial {
				true => ensure!(i == 0, "First accounting period must have is_initial"),
				false => ensure!(i > 0, "Only first accounting period must have is_initial"),
			}
		}

		for pairs in self.accounting_period_results.windows(2) {
			if let [last, current] = pairs {
				if last.is_locked && !current.is_locked {
					bail!("Invalid: Found an open period after locked one.");
				}
			}
		}

		for result in &self.accounting_period_results {
			result.validate()?;
		}

		Ok(())
	}

	pub fn get_accounting_result(&self, name: &YearQuarter) -> Option<&AccountingPeriodResult> {
		self.accounting_period_results
			.iter()
			.find(|accounting_period| &accounting_period.name == name)
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
pub struct AccountingPeriodResult {
	pub name: YearQuarter,
	pub is_initial: bool,
	pub is_locked: bool,
	pub recoupment_vouchers: Vec<Voucher>,
	#[serde(serialize_with = "sorted_map")]
	pub track_distribution_vouchers: HashMap<String, Voucher>,
	#[serde(serialize_with = "sorted_map")]
	pub closing_balances: HashMap<String, BigDecimal>,
}

impl AccountingPeriodResult {
	pub fn validate(&self) -> Result<()> {
		for (account, balance) in &self.closing_balances {
			let account = AccountId::parse(account).unwrap();
			match account {
				AccountId::RevenueTrack(_) => assert!(balance <= 0, "Track revenue must be negative"),
				AccountId::RecoupmentTrack(_) | AccountId::RecoupmentAlbum(_) => ensure!(balance <= 0, "Recoupment balances cannot be posivite ({balance}). If a recoupment was refunded, it must be distributed between the label and artists. It may be that there were no royalties, causing there to not be any distribution voucher created"),
				_ => {}
			}
		}
		Ok(())
	}
	pub fn prev_result<'a>(&self, project: &'a Project) -> Option<&'a AccountingPeriodResult> {
		let prev_name = self.name.get_prev();
		if self.is_initial {
			return None;
		}
		Some(
			project
				.data
				.get_accounting_result(&prev_name)
				.expect(&format!(
					"Could not find previous result {}",
					prev_name.to_string()
				)),
		)
	}
	pub fn get_closing_balances(&self, project: &Project) -> Result<HashMap<String, BigDecimal>> {
		let prev_result = self.prev_result(project);
		let prev_closing_balances = prev_result.map(|r| r.closing_balances.clone());

		let mut closing_balances = prev_closing_balances.unwrap_or_default();

		for (account, balance_change) in self.get_balance_changes() {
			let is_clearing_accout = match account {
				AccountId::RevenueTrack(_) => false,
				AccountId::RecoupmentTrack(_) => false,
				AccountId::RecoupmentAlbum(_) => false,
				AccountId::ExpenseRecoupableTrack(_) => false,
				AccountId::ExpenseRecoupableAlbum(_) => false,
				AccountId::LabelRoyalty => false,
				AccountId::Artist(_) => false,
			};
			if is_clearing_accout {
				// Skip "clearing" accounts, which are intermeadiates just for moving balances around
				ensure!(
					balance_change == BigDecimal::zero(),
					"Clearing account balance must be zero"
				);
				continue;
			}
			let closing_balance = closing_balances
				.entry(account.to_string())
				.or_insert(BigDecimal::zero());
			*closing_balance += &balance_change;
		}

		for (_, balance) in &mut closing_balances {
			// Prevent trailing zeroes
			*balance = balance.normalized();
		}

		Ok(closing_balances)
	}
	pub fn get_balance_changes(&self) -> HashMap<AccountId, BigDecimal> {
		let mut balance_changes = HashMap::new();
		let vouchers = self
			.recoupment_vouchers
			.iter()
			.chain(self.track_distribution_vouchers.values());
		for voucher in vouchers {
			for entry in &voucher.entries {
				let balance_change = balance_changes
					.entry(entry.account.clone())
					.or_insert(BigDecimal::zero());
				*balance_change += &entry.amount;
			}
		}
		balance_changes
	}
	pub fn get_closing_balance(
		&self,
		account: &AccountId,
		project: &Project,
	) -> Option<BigDecimal> {
		let prev_result = self.prev_result(project);
		let prev_closing_balance = prev_result
			.map(|r| r.closing_balances.get(&account.to_string()))
			.flatten();

		let mut closing_balance = prev_closing_balance.cloned();

		let vouchers = self
			.recoupment_vouchers
			.iter()
			.chain(self.track_distribution_vouchers.values());
		for voucher in vouchers {
			for entry in &voucher.entries {
				if &entry.account == account {
					let closing_balance = closing_balance.get_or_insert(BigDecimal::zero());
					*closing_balance += &entry.amount;
				}
			}
		}

		closing_balance
	}
	pub fn get_recoupment_account_associated_with_track(
		&self,
		isrc: &str,
		project: &Project,
	) -> Option<AccountId> {
		let track = project.get_track(isrc).unwrap();

		let album = project.get_album_containing_isrc(isrc);
		let album_recoupment = album.and_then(|a| a.recoupment.as_ref());

		match (&track.recoupment, album_recoupment) {
			(Some(_), None) => Some(AccountId::RecoupmentTrack(track.main_isrc.clone())),
			(None, Some(_)) => Some(AccountId::RecoupmentAlbum(album.unwrap().upc.clone())),
			(None, None) => None,
			(Some(_), Some(_)) => {
				panic!("Track {isrc} has both track and album recoupment accounts")
			}
		}
	}
	pub fn export(&self, project: &Project) -> Export {
		let mut artist_statements = Vec::new();
		for (account, balance) in &self.closing_balances {
			let account_id = AccountId::parse(account).unwrap();
			let artist_name = match account_id {
				AccountId::Artist(name) => name,
				_ => continue,
			};
			let mut artist_statement = ArtistStatementExport {
				payee: artist_name.clone(),
				net_royalties: balance.clone(),
				tracks: Vec::new(),
			};
			for (isrc, voucher) in &self.track_distribution_vouchers {
				let track_account = AccountId::Track(isrc.clone());
				for entry in &voucher.entries {
					match &entry.account {
						AccountId::Artist(n) if n == &artist_name => {
							let gross_royalties = sum_account_vouchers(
								&track_account,
								&[self.revenue_voucher.clone()],
							)
							.expect("Track distribution entry has no track revenue voucher");
							let payable_royalties = voucher
								.entries
								.iter()
								.find(|e| e.account == AccountId::Track(isrc.clone()))
								.expect("No track entry in distribution voucher");
							let track = project.get_track(isrc).unwrap();
							artist_statement.tracks.push(ArtistTrackStatementExport {
								isrc: isrc.clone(),
								title: track.title.clone(),
								gross_royalties: gross_royalties.clone(),
								payable_royalties: payable_royalties.amount.clone(),
								net_royalties: entry.amount.clone(),
							});
						}
						_ => continue,
					}
				}
			}
			assert!(
				artist_statement.net_royalties
					== artist_statement
						.tracks
						.iter()
						.map(|t| &t.net_royalties)
						.sum::<BigDecimal>(),
				"Artist statement net royalties do not match"
			);
			artist_statements.push(artist_statement);
		}
		Export {
			name: self.name.clone(),
			is_initial: self.is_initial,
			artist_statements,
		}
	}
}

#[derive(Serialize)]
pub struct Export {
	name: YearQuarter,
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
	net_royalties: BigDecimal,
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

pub fn compact_elements<T, S>(items: &[T], s: S) -> Result<S::Ok, S::Error>
where
	T: Serialize,
	S: Serializer,
{
	let raw_list: Vec<_> = items
		.iter()
		.map(|i| {
			let json = serde_json::to_string(i).map_err(serde::ser::Error::custom)?;
			RawValue::from_string(json).map_err(serde::ser::Error::custom)
		})
		.collect::<Result<Vec<_>, _>>()?;

	raw_list.serialize(s)
}

#[cfg(test)]
mod test {
	use crate::generator::generate;
	use crate::project::{Project, YearQuarter};
	use crate::project_data::AccountingData;
	use anyhow::Result;
	use pretty_assertions::assert_eq;
	use std::path::PathBuf;

	#[test]
	fn test_generate_artist_statements() -> Result<()> {
		let mut project = Project::load(PathBuf::from("test/Manifest.jsonc"))?;

		let q1_result = generate(&project, &YearQuarter::parse("1999 Q1"))?;
		project.add_result(q1_result)?;

		let q2_result = generate(&project, &YearQuarter::parse("1999 Q2"))?;
		project.add_result(q2_result)?;

		// project.data.save(&project.data_file_path)?;

		let expected_data =
			AccountingData::open(&PathBuf::from("test/Internal data expected.json"))?;

		assert_eq!(
			project.data.accounting_period_results.len(),
			expected_data.accounting_period_results.len(),
		);
		for (i, result) in project.data.accounting_period_results.iter().enumerate() {
			// Serialize to check for trailing zeroes, and for proper diff sorting
			let result_str = serde_json::to_string_pretty(&result).unwrap();
			let expected_srt =
				serde_json::to_string_pretty(&expected_data.accounting_period_results[i]).unwrap();
			assert_eq!(result_str, expected_srt);
		}
		Ok(())
	}
}
