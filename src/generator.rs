use std::{
	collections::{HashMap, HashSet},
	str::FromStr,
};

use crate::{
	accounting::{AccountId, Entry, Voucher},
	project::{Project, YearQuarter},
	project_data::{pct_to_factor, AccountingPeriodResult},
};
use anyhow::{ensure, Context, Result};
use bigdecimal::{BigDecimal, FromPrimitive, Zero};

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

	let mut remaining_tracks: HashMap<String, _> = track_sales_report.tracks.into_iter().collect();
	let recoupment_upcs: HashSet<String> = remaining_tracks
		.keys()
		.filter_map(|isrc| {
			match result.get_recoupment_account_associated_with_track(isrc, &project) {
				Some(AccountId::RecoupmentAlbum(upc)) => Some(upc),
				_ => None,
			}
		})
		.collect();

	// Handle tracks with album recoupments
	for upc in recoupment_upcs {
		let album = project.albums.get(&upc).context("Album not found")?;
		let recoupment_balance =
			result.get_closing_balance(&AccountId::RecoupmentAlbum(upc.clone()), &project);
		let recoupment_balance = match recoupment_balance {
			Some(balance) => balance,
			None => continue,
		};
		let total_album_recoupables = -recoupment_balance;
		let total_album_revenue: BigDecimal = album
			.isrcs
			.iter()
			.map(|isrc| &remaining_tracks.get(isrc).unwrap().gross_royalties)
			.sum();

		if total_album_recoupables.is_zero() {
			continue;
		} else if total_album_recoupables < 0 {
			panic!("Album recoupment balance is not positive. Todo: If it's zero, just don't recoup anything. Is it possible for it to be negative?");
		} else if total_album_revenue <= 0 {
			panic!("Album revenue is not positive. Todo. NOTE: One song could have -1 and another song +1.");
		} else if total_album_recoupables >= total_album_revenue {
			// Recoup 100%
			for isrc in &album.isrcs {
				let track_sales = remaining_tracks.remove(isrc).unwrap();
				let voucher = distribute_track_revenue(
					&result,
					project,
					&isrc,
					&track_sales.gross_royalties,
					&Some(track_sales.gross_royalties.clone()),
				)?;
				result.insert_track_distribution_vouchers_fresh(isrc.clone(), voucher);
			}
		} else if total_album_recoupables < total_album_revenue {
			let mut album_recoup_remainder = total_album_recoupables.clone();
			let recoup_percent = total_album_recoupables / total_album_revenue;

			for (i, isrc) in album.isrcs.iter().enumerate() {
				let is_last = i == album.isrcs.len() - 1;
				let track_sales = remaining_tracks.remove(isrc).unwrap();
				if track_sales.gross_royalties < 0 {
					todo!("Album recoupment processing where a track has negative sales. Not easy to implement. The track's previous royalties might have been recouped, but it also might not have been. The easy thing might be to just not undo anything.")
				}

				// the .round(8) is used for a tolerance check later
				let normal_recoup_amount =
					(&track_sales.gross_royalties * &recoup_percent).round(8);
				let track_recoup_amount = if !is_last {
					album_recoup_remainder -= &normal_recoup_amount;
					normal_recoup_amount
				} else {
					if album_recoup_remainder > track_sales.gross_royalties {
						todo!("Recoupment due to rounding errors exceeds track's gross royalties");
					}
					if album_recoup_remainder != normal_recoup_amount {
						// tolerance is based on the .round(8)
						let diff = (&album_recoup_remainder - &normal_recoup_amount).abs();
						let tolerance = BigDecimal::from_str("0.000000005").unwrap()
							* BigDecimal::from_usize(album.isrcs.len()).unwrap();
						assert!(
							diff <= tolerance,
							"Album recoup remainder diverges from the per-track amount. Code has some calculation issue",
						);
					}
					album_recoup_remainder.clone()
				};

				let voucher = distribute_track_revenue(
					&result,
					project,
					&isrc,
					&track_sales.gross_royalties,
					&Some(track_recoup_amount),
				)?;
				result.insert_track_distribution_vouchers_fresh(isrc.clone(), voucher);
			}
		}
	}

	for (isrc, amount) in remaining_tracks {
		let voucher =
			distribute_track_revenue(&result, project, &isrc, &amount.gross_royalties, &None)?;
		result.insert_track_distribution_vouchers_fresh(isrc, voucher);
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
	let mut sorted_albums: Vec<_> = project.albums.values().collect();
	sorted_albums.sort_by_key(|a| &a.upc);
	for album in sorted_albums {
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
	recoup_amount_from_album: &Option<BigDecimal>,
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

	// Track-level recoupment
	let recoupment_account = result.get_recoupment_account_associated_with_track(isrc, project);
	if let Some(recoupment_account) = recoupment_account {
		let recoupment_balance = result.get_closing_balance(&recoupment_account, project);
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
				AccountId::RecoupmentAlbum(upc) => {
					let recoupables_from_album = recoup_amount_from_album.clone().unwrap();
					ensure!(recoupables_from_album <= remaining);
					ensure!(remaining >= 0, "Todo: Negative royalties received when there's an album recoupment. In this case I think recoupments should be reverted, in order to not impact other tracks in the album.");
					if recoupables_from_album < 0 {
						// The recoupables were negative, for example the expense was already recouped, but then refunded.
						todo!("Negative recoupables. How should that be handled?")
					} else if remaining > 0 && recoupables_from_album > 0 {
						remaining -= &recoupables_from_album;
						entries.push(Entry {
							account: AccountId::RecoupmentAlbum(upc),
							amount: recoupables_from_album,
							note: None,
						});
					}
				}
				_ => panic!(),
			}
		}
	}

	// Split track royalties between the label and artists. It's not possible to
	// have a remainder here because the splits are multiplicative (e.g 1/3 can't happen)
	let splittable_royalties = remaining;

	if splittable_royalties != BigDecimal::zero() {
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
			});
		}
	}

	let voucher = Voucher::new_validated(entries)?;
	Ok(voucher)
}
