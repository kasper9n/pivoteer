use crate::{
	accounting::{Entry, EntryKind, Voucher},
	manifest::Track,
	project::{AccountingPeriod, Project},
	project_data::AccountingPeriodResult,
	track_sales_report::TrackSalesReport,
};
use anyhow::{ensure, Context, Result};
use bigdecimal::BigDecimal;

pub fn generate(project: &mut Project) -> Result<()> {
	let open_periods: Vec<_> = project
		.accounting_periods
		.iter()
		.filter(|p| !project.data.accounts.is_period_closed(&p.name))
		.collect();

	if open_periods.is_empty() {
		println!("No open periods to process. All periods are closed.");
		return Ok(());
	}
	println!("Processing {} open period(s)", open_periods.len());

	// 4. Process each open period
	for period in open_periods {
		println!("Processing period: {}", period.name);
		process_period(&mut project, &period)?;
	}

	println!("✓ Generation complete");
	Ok(())
}

fn process_period(
	project: &mut Project,
	period: &AccountingPeriod,
) -> Result<AccountingPeriodResult> {
	let mut result = AccountingPeriodResult {
		name: period.name.clone(),
		is_closed: false,
		is_initial: period.is_initial,
		vouchers: Vec::new(),
	};

	let sales_report = period.generate_sales_report();
	let track_sales_report = sales_report.into_track_sales_report(&project);

	let revenue_voucher = create_revenue_voucher(&mut *project, &period, &track_sales_report)?;
	result.vouchers.push(revenue_voucher);

	for (isrc, amount) in track_sales_report {
		ensure!(amount != 0, "Track {isrc} has zero revenue. It's probably just an earnings adjustment, so this check can probably be removed");
		let track_voucher = distribute_track_revenue(project, period, isrc, amount)?;
		result.vouchers.push(track_voucher);
	}

	Ok(result)
}

fn create_revenue_voucher(
	project: &mut Project,
	period: &AccountingPeriod,
	track_sales_report: &TrackSalesReport,
) -> Result<Voucher> {
	let mut accounts = project.data.accounts;

	let mut voucher = Voucher {
		id: project.data.generate_voucher_id(),
		date: period.end_date(),
		entries: Vec::new(),
		note: "Revenue".to_string(),
	};

	let revenue_entry = Entry {
		account_id: accounts.get_revenue_account().id,
		amount: track_sales_report
			.tracks
			.iter()
			.map(|(_, t)| t.gross_royalties)
			.sum(),
		note: "".to_string(),
		kind: EntryKind::Debit,
	};
	voucher.entries.push(revenue_entry);

	for (_, track) in track_sales_report.tracks {
		let track_account_id = accounts.get_or_create_track_account(&track.isrc);
		let track_entry = Entry {
			account_id: track_account_id,
			amount: track.gross_royalties,
			note: format!(""),
			kind: EntryKind::Credit,
		};
		voucher.entries.push(track_entry);
	}

	voucher.verify_balance()?;
	Ok(voucher)
}

fn distribute_track_revenue(
	project: &mut Project,
	period: &AccountingPeriod,
	isrc: &str,
	revenue: &BigDecimal,
) -> Result<Voucher> {
	let track = project.get_track(isrc).context("Track not found")?;

	let track_account_id = project.data.accounts.get_or_create_track_account(isrc);
	let mut remaining = revenue.clone();

	let album = project.get_album_containing_isrc(isrc.into());

	// Recoup expenses if track has an expense account
	if let Some(expense_account) = project.data.accounts.get_expense_account(isrc, &project) {
		// Recoupments are assets, so they are negative
		let expense_balance = expense_account
			.account()
			.closing_balance_at(project, period.end_date());
	}

	// Step 2: Distribute remaining to artists (considering label_share)
	if remaining > BigDecimal::from(0) {
		distribute_to_artists(project, period, track, isrc, &remaining)?;
	}

	Ok(())
}

fn calculate_expense_payment(available: &BigDecimal, track: &Track) -> Result<BigDecimal> {
	// How much more can we recoup?
	let remaining_to_recoup = track.max_recoup - already_recouped;

	if remaining_to_recoup <= BigDecimal::from(0) {
		// Already hit the recoup limit
		return Ok(BigDecimal::from(0));
	}

	// Pay the minimum of: what's available, what remains to recoup
	let payment = if available >= &remaining_to_recoup {
		remaining_to_recoup
	} else {
		available.clone()
	};

	Ok(payment)
}

fn distribute_to_artists(
	project: &mut Project,
	period: &AccountingPeriod,
	track: &Track,
	isrc: &str,
	amount: &BigDecimal,
) -> Result<()> {
	let track_account_id = project.data.accounts.get_or_create_track_account(isrc);

	// Calculate label and artist shares
	let label_share_pct = &track.label_share;
	let artist_share_pct = BigDecimal::from(100) - label_share_pct;

	let label_amount = amount * label_share_pct / BigDecimal::from(100);
	let artist_pool = amount * &artist_share_pct / BigDecimal::from(100);

	let mut entries = Vec::new();

	// Debit from track account (total going out)
	entries.push(Entry {
		account_id: track_account_id,
		amount: amount.clone(),
		note: format!("Distribution for {}", track.title),
		kind: EntryKind::Debit,
	});

	// Credit to label
	if label_amount > BigDecimal::from(0) {
		let label_account_id = project.data.accounts.get_or_create_label_account();
		entries.push(Entry {
			account_id: label_account_id,
			amount: label_amount.clone(),
			note: format!("{}% label share for {}", label_share_pct, track.title),
			kind: EntryKind::Credit,
		});
	}

	// Credit to each artist based on their split of the artist pool
	for split in &track.splits {
		let artist_amount = &artist_pool * &split.share / BigDecimal::from(100);
		let artist_account_id = project
			.data
			.accounts
			.get_or_create_artist_account(&split.name);

		entries.push(Entry {
			account_id: artist_account_id,
			amount: artist_amount,
			note: format!("{}% artist split for {}", split.share, track.title),
			kind: EntryKind::Credit,
		});
	}

	let voucher = Voucher::new(
		project.data.generate_voucher_id(),
		period.end_date(),
		entries,
		format!("Artist/Label distribution for ISRC {}", isrc),
	)?;

	project.data.add_voucher(&period.name, voucher)?;

	Ok(())
}
