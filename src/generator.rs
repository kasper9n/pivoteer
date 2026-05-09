use std::collections::HashMap;

use crate::{
	accounting::{AccountId, Entry, Voucher},
	project::{Project, YearQuarter},
	project_data::{pct_to_factor, AccountingPeriodResult},
};
use anyhow::{Context, Result};
use bigdecimal::{BigDecimal, Zero};

pub fn generate(project: &Project, pname: &YearQuarter) -> Result<AccountingPeriodResult> {
	let period = project.get_accounting_period(&pname).unwrap();
	let sales_report = period.generate_sales_report();
	let track_sales_report = sales_report.into_track_sales_report(&project);

	let recoupment_vouchers = create_recoupment_vouchers(&pname, &project)?;

	let period = project.get_accounting_period(&pname).unwrap();
	let mut result = AccountingPeriodResult {
		name: pname.clone(),
		is_initial: period.is_initial,
		is_locked: false,
		recoupment_vouchers,
		track_distribution_vouchers: HashMap::new(),
		closing_balances: HashMap::new(),
	};

	for (isrc, amount) in track_sales_report.tracks {
		let voucher = distribute_track_revenue(&result, project, &isrc, &amount.gross_royalties)?;
		let replaced = result.track_distribution_vouchers.insert(isrc, voucher);
		assert!(
			replaced.is_none(),
			"Duplicate of track distribution voucher {:?}",
			replaced
		);
	}

	result.closing_balances = result.get_closing_balances(&project)?;
	result.validate()?;

	Ok(result)
}

fn create_recoupment_vouchers(pname: &YearQuarter, project: &Project) -> Result<Vec<Voucher>> {
	let mut recoupment_vouchers: Vec<Voucher> = Vec::new();
	for track in &project.tracks {
		let recoupment_manifest = match &track.recoupment {
			Some(v) => v,
			None => continue,
		};
		for recoupment in &recoupment_manifest.recoupments {
			if pname.contains_date(&recoupment.date) {
				let entries = vec![
					Entry {
						account: AccountId::RecoupmentTrack(track.main_isrc.clone()),
						amount: -recoupment.recoup.clone(),
						note: None,
					},
					Entry {
						account: AccountId::ExpenseRecoupableTrack(track.main_isrc.clone()),
						amount: recoupment.recoup.clone(),
						note: None,
					},
				];
				let voucher = Voucher::new_validated(entries)?;
				recoupment_vouchers.push(voucher);
			}
		}
	}
	for album in project.albums.values() {
		let recoupment_manifest = match &album.recoupment {
			Some(v) => v,
			None => continue,
		};
		for recoupment in &recoupment_manifest.recoupments {
			if pname.contains_date(&recoupment.date) {
				let entries = vec![
					Entry {
						account: AccountId::RecoupmentAlbum(album.upc.clone()),
						amount: -recoupment.recoup.clone(),
						note: None,
					},
					Entry {
						account: AccountId::ExpenseRecoupableAlbum(album.upc.clone()),
						amount: recoupment.recoup.clone(),
						note: None,
					},
				];
				let voucher = Voucher::new_validated(entries)?;
				recoupment_vouchers.push(voucher);
			}
		}
	}
	Ok(recoupment_vouchers)
}

fn distribute_track_revenue(
	result: &AccountingPeriodResult,
	project: &Project,
	isrc: &str,
	revenue: &BigDecimal,
) -> Result<Voucher> {
	let track = project.get_track(isrc).context("Track not found")?;

	let mut entries = Vec::new();

	let track_account_id = AccountId::RevenueTrack(isrc.to_string());
	track_account_id.validate()?;
	entries.push(Entry {
		account: track_account_id,
		amount: -revenue.clone(),
		note: None,
	});

	let mut remaining = revenue.clone();

	// Recoup recoupable expenses
	let recoupment_account = result.get_recoupment_account_associated_with_track(&isrc, &project);
	if let Some(recoupment_account) = recoupment_account {
		let recoupment_balance = result.get_closing_balance(&recoupment_account, &project);
		if let Some(recoupment_balance) = recoupment_balance {
			match recoupment_account {
				AccountId::RecoupmentTrack(_) => {
					// Recoupment balance is generally negative because it's a future receivable amount
					let recoupables = -recoupment_balance;
					if recoupables < 0 {
						// The recoupables were negative, for example the expense was already recouped, but then refunded.
						remaining -= &recoupables;
						entries.push(Entry {
							account: AccountId::RecoupmentTrack(isrc.to_string()),
							amount: recoupables,
							note: None,
						});
					} else if remaining > 0 && recoupables > 0 {
						// There is an amount that can be recouped
						let amount_to_recoup = BigDecimal::min(remaining.clone(), recoupables);
						remaining -= &amount_to_recoup;
						entries.push(Entry {
							account: AccountId::RecoupmentTrack(isrc.to_string()),
							amount: amount_to_recoup,
							note: None,
						});
					}
				}
				AccountId::RecoupmentAlbum(_) => todo!("Album recoupment not implemented yet. When implemented: If a song with 3 tracks gets 10$ in royalties, we can distribute 3.33$ to each artist and keep a 0.01$ in the track account."),
				_ => panic!(),
			}
		}
	}

	let splittable_royalties = remaining;

	// Allow both positive royalties and negative royalty adjustments
	if splittable_royalties != BigDecimal::zero() {
		// Distribute remaining to artists and label
		entries.push(Entry {
			account: AccountId::LabelRoyalty,
			amount: splittable_royalties.clone() * pct_to_factor(&track.label_share),
			note: None,
		});
		let artists_share = BigDecimal::from(100) - &track.label_share;
		let artists_splittable_royalties = splittable_royalties * pct_to_factor(&artists_share);
		for split in &track.splits {
			entries.push(Entry {
				account: AccountId::Artist(split.name.clone()),
				amount: &artists_splittable_royalties * pct_to_factor(&split.share),
				note: None,
			})
		}
	}

	let voucher = Voucher::new_validated(entries)?;
	Ok(voucher)
}
