use crate::{project::Project, project_data::sorted_map};
use anyhow::{ensure, Result};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Accounts {
	#[serde(serialize_with = "sorted_map")]
	accounts: HashMap<String, Account>,
}
impl Accounts {
	pub fn new() -> Self {
		let mut accounts = std::collections::HashMap::new();

		// Global revenue account
		accounts.insert(
			"revenue".to_string(),
			Account {
				id: "revenue".to_string(),
				name: "Royalty Revenue".to_string(),
				kind: AccountKind::Revenue,
			},
		);

		Self { accounts }
	}

	pub fn get_revenue_account(&self) -> &Account {
		self.accounts.get("revenue").unwrap()
	}

	pub fn get_track_account(&self, isrc: &str) -> Option<&Account> {
		self.accounts.get(&format!("track:{isrc}"))
	}

	pub fn get_or_create_track_account(&mut self, isrc: &str) -> String {
		let id = format!("track:{isrc}");
		self.accounts.entry(id.clone()).or_insert_with(|| Account {
			id: id.clone(),
			name: format!("Track {}", isrc),
			kind: AccountKind::TrackAsset,
		});
		id
	}

	pub fn get_expense_account(&mut self, isrc: &str, project: &Project) -> Option<ExpenseAccount> {
		let album = project.get_album_containing_isrc(isrc);
		let upc = album.map(|album| album.upc);
		let isrc_expense_account = self.accounts.get(&format!("expense:isrc:{isrc}"));
		let upc_expense_account = self.accounts.get(&format!("expense:upc:{upc}"));
		match (isrc_expense_account, upc_expense_account) {
			(Some(isrc_account), None) => Some(ExpenseAccount::Track(isrc_account)),
			(None, Some(upc_account)) => Some(ExpenseAccount::Album(upc_account)),
			(None, None) => None,
			_ => panic!("Track {isrc} has both track and album expense accounts"),
		}
	}

	pub fn get_artist_account(&self, artist: &str) -> Option<&Account> {
		self.accounts.get(&format!("artist:{artist}"))
	}

	pub fn get_or_create_artist_account(&mut self, artist: &str) -> String {
		let id = format!("artist:{artist}");
		self.accounts.entry(id.clone()).or_insert_with(|| Account {
			id: id.clone(),
			name: format!("Artist Payable: {}", artist),
			kind: AccountKind::ArtistLiability,
		});
		id
	}
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Accounting {
	pub vouchers: Vec<Voucher>,
}

impl Accounting {
	pub fn new() -> Self {
		Self { vouchers: vec![] }
	}

	pub fn add_voucher(&mut self, voucher: Voucher) {
		self.vouchers.push(voucher);
	}

	pub fn get_vouchers(&self) -> &Vec<Voucher> {
		&self
	}
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum AccountKind {
	Revenue,         // Global revenue account
	TrackAsset,      // Track accounts (asset/clearing account)
	Expense,         // Recoupment expense accounts
	ArtistLiability, // Artist payable accounts
}

enum ExpenseAccount<'a> {
	Track(&'a Account),
	Album(&'a Account),
}
impl ExpenseAccount<'_> {
	pub fn account(&self) -> &Account {
		match self {
			ExpenseAccount::Track(account) => account,
			ExpenseAccount::Album(account) => account,
		}
	}
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Account {
	pub id: String,
	name: String,
	kind: AccountKind,
}
impl Account {
	pub fn closing_balance_at(&self, project: &Project, date: NaiveDate) -> Result<BigDecimal> {
		let mut balance = BigDecimal::from(0);
		for voucher in &project.data.vouchers {
			if voucher.date > date {
				break;
			}
			for entry in &voucher.entries {
				if entry.account_id == self.id {
					balance += &entry.amount;
				}
			}
		}
		Ok(balance)
	}
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Entry {
	pub account_id: String,
	pub amount: BigDecimal,
	pub note: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Voucher {
	pub id: u32,
	pub date: NaiveDate,
	pub entries: Vec<Entry>,
	pub note: String,
}

impl Voucher {
	pub fn new(id: u32, date: NaiveDate, entries: Vec<Entry>, note: String) -> Result<Self> {
		ensure!(entries.len() >= 2, "Voucher must have at least 2 entries");
		Ok(Voucher {
			id,
			date,
			entries,
			note,
		})
	}

	pub fn verify_balance(&self) -> Result<()> {
		let mut total = BigDecimal::from(0);
		for entry in &self.entries {
			total += &entry.amount;
		}
		ensure!(total == 0, "Voucher not in balance: {:#?}", self);
		Ok(())
	}
}
