use crate::project_data::compact_elements;
use anyhow::{bail, ensure, Result};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq)]
pub enum AccountId {
	Revenue,
	Track(String),
	RecoupmentTrack(String),
	RecoupmentAlbum(String),
	RecoupmentExpense,
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
		} else if s == "recoupment_expense" {
			Ok(AccountId::RecoupmentExpense)
		} else if s == "label_royalty" {
			Ok(AccountId::LabelRoyalty)
		} else if let Some(name) = s.strip_prefix("artist:") {
			Ok(AccountId::Artist(name.to_string().replace(':', " ")))
		} else {
			bail!("Invalid account ID: {s}");
		}
	}
	pub fn validate(&self) -> Result<()> {
		match self {
			AccountId::Track(isrc) => ensure!(is_valid_isrc(isrc), "Invalid ISRC: {isrc}"),
			AccountId::RecoupmentTrack(isrc) => {
				ensure!(is_valid_isrc(isrc), "Invalid ISRC: {isrc}")
			}
			AccountId::RecoupmentAlbum(upc) => ensure!(is_valid_upc(upc), "Invalid UPC: {upc}"),
			AccountId::Revenue => {}
			AccountId::RecoupmentExpense => {}
			AccountId::LabelRoyalty => {}
			AccountId::Artist(_) => {}
		}
		Ok(())
	}
}
impl ToString for AccountId {
	fn to_string(&self) -> String {
		match self {
			AccountId::Revenue => format!("revenue"),
			AccountId::Track(isrc) => format!("track:{}", isrc),
			AccountId::RecoupmentTrack(isrc) => format!("recoupment:track:{}", isrc),
			AccountId::RecoupmentAlbum(upc) => format!("recoupment:album:{}", upc),
			AccountId::RecoupmentExpense => format!("recoupment_expense"),
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

pub fn is_valid_isrc(isrc: &str) -> bool {
	// Standard ISRC: 12 characters (e.g., USABC1234567)
	// 2 alpha (country), 3 alpha-numeric (registrant), 2 digit (year), 5 digit (id)
	isrc.len() == 12
		&& isrc.chars().take(2).all(|c| c.is_ascii_alphabetic())
		&& isrc.chars().skip(2).all(|c| c.is_ascii_alphanumeric())
}

pub fn is_valid_upc(upc: &str) -> bool {
	// Standard UPC-A is 12 digits, EAN-13 is 13 digits.
	// Most music distributors use the 12 or 13-digit format.
	(upc.len() == 12 || upc.len() == 13) && upc.chars().all(|c| c.is_ascii_digit())
}

// pub struct Balance {
// 	pub account: AccountId,
// 	pub amount: BigDecimal,
// }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Entry {
	pub account: AccountId,
	pub amount: BigDecimal,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub note: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(transparent)]
pub struct Voucher {
	#[serde(serialize_with = "compact_elements")]
	pub entries: Vec<Entry>,
}

impl Voucher {
	pub fn new_validated(entries: Vec<Entry>) -> Result<Self> {
		let voucher = Voucher { entries };
		voucher.validate()?;
		Ok(voucher)
	}

	pub fn validate(&self) -> Result<()> {
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
