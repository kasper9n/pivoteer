use crate::{
	accounting::{AccountId, Entry, Voucher},
	project::{Project, YearQuarter},
	project_data::{pct_to_factor, AccountingPeriodResult},
	track_sales_report::TrackSalesReport,
};
use anyhow::{Context, Result};
use bigdecimal::{BigDecimal, Zero};

pub fn generate(project: &mut Project, pname: &YearQuarter) -> Result<AccountingPeriodResult> {
	let period = project.get_accounting_period(&pname).unwrap();
	let sales_report = period.generate_sales_report();
	let track_sales_report = sales_report.into_track_sales_report(&project);
	let revenue_voucher = create_revenue_voucher(&track_sales_report, &pname, &mut *project)?;

	let recoupment_vouchers = create_recoupment_vouchers(&pname, &mut *project)?;

	let period = project.get_accounting_period(&pname).unwrap();
	let mut result = AccountingPeriodResult {
		name: pname.clone(),
		is_initial: period.is_initial,
		is_locked: false,
		revenue_voucher,
		recoupment_vouchers,
		track_distribution_vouchers: Vec::new(),
		closing_revenue_balances: todo!(),
		closing_recoupment_balances: todo!(),
		closing_artist_balances: todo!(),
	};

	let mut track_distribution_vouchers = result.track_distribution_vouchers;
	for (isrc, amount) in track_sales_report.tracks {
		let voucher =
			distribute_track_revenue(&mut result, &mut project, &isrc, &amount.gross_royalties)?;
		track_distribution_vouchers.push(voucher);
	}

	Ok(result)
}

fn create_revenue_voucher(
	track_sales_report: &TrackSalesReport,
	pname: &YearQuarter,
	project: &mut Project,
) -> Result<Voucher> {
	let revenue_entry = Entry {
		account: AccountId::Revenue,
		amount: -track_sales_report
			.tracks
			.iter()
			.map(|(_, t)| &t.gross_royalties)
			.sum::<BigDecimal>(),
		note: None,
	};

	let mut entries = vec![revenue_entry];

	for (_, track) in &track_sales_report.tracks {
		let track_account_id = AccountId::Track(track.isrc.clone());
		let track_entry = Entry {
			account: track_account_id,
			amount: track.gross_royalties.clone(),
			note: None,
		};
		entries.push(track_entry);
	}

	let voucher = Voucher::new_validated(
		project.data.generate_voucher_id(),
		pname.end_date(),
		entries,
		None,
	)?;
	Ok(voucher)
}

fn create_recoupment_vouchers(pname: &YearQuarter, project: &mut Project) -> Result<Vec<Voucher>> {
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
						account: AccountId::Expense,
						amount: recoupment.expense.clone(),
						note: None,
					},
				];
				let voucher = Voucher::new_validated(
					project.data.generate_voucher_id(),
					pname.end_date(),
					entries,
					None,
				)?;
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
						account: AccountId::Expense,
						amount: recoupment.expense.clone(),
						note: None,
					},
				];
				let voucher = Voucher::new_validated(
					project.data.generate_voucher_id(),
					pname.end_date(),
					entries,
					None,
				)?;
				recoupment_vouchers.push(voucher);
			}
		}
	}
	Ok(recoupment_vouchers)
}

fn distribute_track_revenue(
	result: &mut AccountingPeriodResult,
	project: &mut Project,
	isrc: &str,
	revenue: &BigDecimal,
) -> Result<Voucher> {
	let track = project.get_track(isrc).context("Track not found")?;

	let mut entries = Vec::new();

	let track_account_id = AccountId::Track(isrc.to_string());
	track_account_id.validate()?;
	entries.push(Entry {
		account: track_account_id,
		amount: -revenue.clone(),
		note: None,
	});

	let mut remaining = revenue.clone();

	// Recoup recoupable expenses
	let recoupment_balance = result
		.get_recoupment_account_associated_with_track(&isrc, &project)
		.and_then(|recoupment_account| {
			result.get_closing_recoupment_balance(&recoupment_account, &project)
		});
	if let Some(recoupment_balance) = recoupment_balance {
		match recoupment_balance.account {
			AccountId::Track(_) => {
				// Recoupment balance is negative because it's a future receivable amount
				let recoupables = -recoupment_balance.amount;
				if remaining > 0 && recoupables > 0 {
					let amount_to_recoup = BigDecimal::min(remaining.clone(), recoupables);
					remaining -= &amount_to_recoup;
					entries.push(Entry {
						account: AccountId::RecoupmentTrack(isrc.to_string()),
						amount: amount_to_recoup,
						note: None,
					});
				}
			}
			AccountId::RecoupmentAlbum(_) => todo!("Album recoupment not implemented yet"),
			_ => panic!(),
		};
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
		let artists_splittable_royalties = splittable_royalties * artists_share;
		for split in &track.splits {
			entries.push(Entry {
				account: AccountId::Artist(split.name.clone()),
				amount: &artists_splittable_royalties * pct_to_factor(&split.share),
				note: None,
			})
		}
	}

	let voucher = Voucher {
		id: project.data.generate_voucher_id(),
		date: result.end_date(),
		entries,
		note: None,
	};
	Ok(voucher)
}
