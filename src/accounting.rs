use crate::project_data::compact_elements;
use anyhow::{bail, ensure, Result};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum AccountId {
	RevenueTrack(String),
	RecoupmentTrack(String),
	RecoupmentAlbum(String),
	ExpenseRecoupableTrack(String),
	ExpenseRecoupableAlbum(String),
	LabelRoyalty,
	Artist(String),
}
impl AccountId {
	pub fn parse(s: &str) -> Result<Self> {
		if let Some(isrc) = s.strip_prefix("revenue:track:") {
			Ok(AccountId::RevenueTrack(isrc.to_string()))
		} else if let Some(isrc) = s.strip_prefix("recoupment:track:") {
			Ok(AccountId::RecoupmentTrack(isrc.to_string()))
		} else if let Some(upc) = s.strip_prefix("recoupment:album:") {
			Ok(AccountId::RecoupmentAlbum(upc.to_string()))
		} else if let Some(isrc) = s.strip_prefix("expense:recoupable:track:") {
			Ok(AccountId::ExpenseRecoupableTrack(isrc.to_string()))
		} else if let Some(isrc) = s.strip_prefix("expense:recoupable:album:") {
			Ok(AccountId::ExpenseRecoupableAlbum(isrc.to_string()))
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
			AccountId::RevenueTrack(isrc) => validte_isrc(isrc),
			AccountId::RecoupmentTrack(isrc) => validte_isrc(isrc),
			AccountId::RecoupmentAlbum(upc) => validate_upc(upc),
			AccountId::ExpenseRecoupableTrack(isrc) => validte_isrc(isrc),
			AccountId::ExpenseRecoupableAlbum(upc) => validate_upc(upc),
			AccountId::LabelRoyalty => Ok(()),
			AccountId::Artist(_) => Ok(()),
		}
	}
}
impl ToString for AccountId {
	fn to_string(&self) -> String {
		match self {
			AccountId::RevenueTrack(isrc) => format!("revenue:track:{isrc}"),
			AccountId::RecoupmentTrack(isrc) => format!("recoupment:track:{isrc}"),
			AccountId::RecoupmentAlbum(upc) => format!("recoupment:album:{upc}"),
			AccountId::ExpenseRecoupableTrack(isrc) => format!("expense:recoupable:track:{isrc}"),
			AccountId::ExpenseRecoupableAlbum(isrc) => format!("expense:recoupable:album:{isrc}"),
			AccountId::LabelRoyalty => format!("label_royalty"),
			AccountId::Artist(name) => format!("artist:{name}"),
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

pub fn validte_isrc(isrc: &str) -> Result<()> {
	// Standard ISRC: 12 characters (e.g., USABC1234567)
	// 2 alpha (country), 3 alpha-numeric (registrant), 2 digit (year), 5 digit (id)
	let is_valid = isrc.len() == 12
		&& isrc.chars().take(2).all(|c| c.is_ascii_alphabetic())
		&& isrc.chars().skip(2).all(|c| c.is_ascii_alphanumeric());
	ensure!(is_valid, "Invalid ISRc: {isrc}");
	Ok(())
}

pub fn validate_upc(upc: &str) -> Result<()> {
	// Standard UPC-A is 12 digits, EAN-13 is 13 digits.
	// Most music distributors use the 12 or 13-digit format.
	let is_valid = (upc.len() == 12 || upc.len() == 13) && upc.chars().all(|c| c.is_ascii_digit());
	ensure!(is_valid, "Invalid UPC: {upc}");
	Ok(())
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
	pub fn new_validated(mut entries: Vec<Entry>) -> Result<Self> {
		for entry in &mut entries {
			// Prevent trailing zeroes
			entry.amount = entry.amount.normalized();
		}
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
