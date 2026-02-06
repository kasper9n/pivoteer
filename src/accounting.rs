use anyhow::{bail, ensure, Result};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq)]
pub enum AccountId {
	Revenue,
	Track(String),
	RecoupmentTrack(String),
	RecoupmentAlbum(String),
	Expense,
	LabelRoyalty,
	Artist(String),
}
impl AccountId {
	pub fn parse(s: &str) -> Result<Self> {
		if s == "revenue" {
			Ok(AccountId::Revenue)
		} else if let Some(isrc) = s.strip_prefix("track:") {
			Ok(AccountId::Track(isrc.to_string()))
		} else if let Some(isrc) = s.strip_prefix("recoupment:track:") {
			Ok(AccountId::RecoupmentTrack(isrc.to_string()))
		} else if let Some(upc) = s.strip_prefix("recoupment:album:") {
			Ok(AccountId::RecoupmentAlbum(upc.to_string()))
		} else if s == "expense" {
			Ok(AccountId::Expense)
		} else if s == "label_royalty" {
			Ok(AccountId::LabelRoyalty)
		} else if let Some(name) = s.strip_prefix("artist:") {
			Ok(AccountId::Artist(name.to_string().replace(':', " ")))
		} else {
			bail!("Invalid account ID: {s}");
		}
	}
	pub fn validate(&self) -> Result<()> {
		todo!("Validate ISRCs and UPCs, maybe even artist name");
	}
	pub fn recoupment_account_id(&self) -> String {
		let full_id = self.to_string();
		match self {
			AccountId::RecoupmentTrack(_) | AccountId::RecoupmentAlbum(_) => {
				full_id.strip_prefix("recoupment:").unwrap().to_string()
			}
			_ => panic!("Not a recoupment account"),
		}
	}
	pub fn artist_account_id(&self) -> String {
		let full_id = self.to_string();
		match self {
			AccountId::Artist(_) => full_id.strip_prefix("artist:").unwrap().to_string(),
			_ => panic!("Not an artist account"),
		}
	}
}
impl ToString for AccountId {
	fn to_string(&self) -> String {
		match self {
			AccountId::Revenue => format!("revenue"),
			AccountId::Track(isrc) => format!("track:{}", isrc),
			AccountId::RecoupmentTrack(isrc) => format!("recoupment:track:{}", isrc),
			AccountId::RecoupmentAlbum(upc) => format!("recoupment:album:{}", upc),
			AccountId::Expense => format!("expense"),
			AccountId::LabelRoyalty => format!("label_royalty"),
			AccountId::Artist(name) => format!("artist:{}", name),
		}
	}
}
impl Serialize for AccountId {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = self.to_string();
		serializer.serialize_str(&s)
	}
}
impl<'de> Deserialize<'de> for AccountId {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		match Self::parse(s.as_str()) {
			Ok(account_id) => Ok(account_id),
			Err(err) => Err(serde::de::Error::custom(err)),
		}
	}
}

pub struct Balance {
	pub account: AccountId,
	pub amount: BigDecimal,
}

// #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
// pub struct Accounts {
// 	#[serde(serialize_with = "sorted_map")]
// 	accounts: HashMap<String, Account>,
// }
// impl Accounts {
// 	pub fn new() -> Self {
// 		let mut accounts = std::collections::HashMap::new();

// 		// Global revenue account
// 		accounts.insert(
// 			"revenue".to_string(),
// 			Account {
// 				id: "revenue".to_string(),
// 				name: "Royalty Revenue".to_string(),
// 				kind: AccountKind::Revenue,
// 			},
// 		);

// 		Self { accounts }
// 	}

// 	pub fn get_revenue_account(&self) -> &Account {
// 		self.accounts.get("revenue").unwrap()
// 	}

// 	pub fn get_track_account(&self, isrc: &str) -> Option<&Account> {
// 		self.accounts.get(&format!("track:{isrc}"))
// 	}

// 	pub fn get_or_create_track_account(&mut self, isrc: &str) -> String {
// 		let id = format!("track:{isrc}");
// 		self.accounts.entry(id.clone()).or_insert_with(|| Account {
// 			id: id.clone(),
// 			name: format!("Track {}", isrc),
// 			kind: AccountKind::TrackAsset,
// 		});
// 		id
// 	}

// 	pub fn get_artist_account(&self, artist: &str) -> Option<&Account> {
// 		self.accounts.get(&format!("artist:{artist}"))
// 	}

// 	pub fn get_or_create_artist_account(&mut self, artist: &str) -> String {
// 		let id = format!("artist:{artist}");
// 		self.accounts.entry(id.clone()).or_insert_with(|| Account {
// 			id: id.clone(),
// 			name: format!("Artist Payable: {}", artist),
// 			kind: AccountKind::ArtistLiability,
// 		});
// 		id
// 	}
// }

// #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
// pub struct Account {
// 	pub id: String,
// 	name: String,
// 	kind: AccountKind,
// }
// impl Account {
// 	pub fn closing_balance_at(&self, project: &Project, date: NaiveDate) -> Result<BigDecimal> {
// 		let mut balance = BigDecimal::from(0);
// 		for voucher in &project.data.vouchers {
// 			if voucher.date > date {
// 				break;
// 			}
// 			for entry in &voucher.entries {
// 				if entry.account_id == self.id {
// 					balance += &entry.amount;
// 				}
// 			}
// 		}
// 		Ok(balance)
// 	}
// }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Entry {
	pub account: AccountId,
	pub amount: BigDecimal,
	pub note: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Voucher {
	pub id: u32,
	pub date: NaiveDate,
	pub entries: Vec<Entry>,
	pub note: Option<String>,
}

impl Voucher {
	pub fn new_validated(
		id: u32,
		date: NaiveDate,
		entries: Vec<Entry>,
		note: Option<String>,
	) -> Result<Self> {
		ensure!(entries.len() >= 2, "Voucher must have at least 2 entries");
		let voucher = Voucher {
			id,
			date,
			entries,
			note,
		};
		voucher.validate()?;
		Ok(voucher)
	}

	pub fn validate(&self) -> Result<()> {
		ensure!(
			self.entries.len() >= 2,
			"Voucher must have at least 2 entries"
		);
		let mut total = BigDecimal::from(0);
		for entry in &self.entries {
			total += &entry.amount;
		}
		ensure!(total == 0, "Voucher not in balance: {:#?}", self);
		Ok(())
	}
}
pub fn sum_account_vouchers(account: &AccountId, vouchers: &[Voucher]) -> Option<BigDecimal> {
	let mut account_exists = false;
	let mut sum = BigDecimal::from(0);
	for voucher in vouchers {
		for entry in &voucher.entries {
			if entry.account == *account {
				account_exists = true;
				sum += &entry.amount;
			}
		}
	}
	match account_exists {
		true => Some(sum),
		false => None,
	}
}
