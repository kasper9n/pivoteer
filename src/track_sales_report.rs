use crate::project::{Project, SalesReport, YearQuarter};
use bigdecimal::{BigDecimal, FromPrimitive, One, Zero};
use std::{collections::HashMap, str::FromStr};

#[derive(Debug)]
pub struct TrackSalesReportRow {
	#[allow(unused)]
	pub isrc: String,
	#[allow(unused)]
	pub title: String,
	pub gross_royalties: BigDecimal,
}
#[derive(Debug)]
pub struct TrackSalesReport {
	pub tracks: HashMap<String, TrackSalesReportRow>,
	#[allow(unused)]
	pub accounting_period_name: YearQuarter,
}

impl TrackSalesReport {
	pub fn from_sales_report(sales_report: SalesReport, project: &Project) -> Self {
		let isrc_report_map = sales_report.isrc_map;
		let mut tracks_map = HashMap::new();

		for (unmapped_isrc, gross_royalties) in isrc_report_map.clone() {
			let track = match project.get_track_by_any_isrc(&unmapped_isrc) {
				Some(val) => val,
				None => panic!(
					"No track with ISRC {}. Was scanning sales report {}",
					unmapped_isrc,
					sales_report.accounting_period_name.to_string()
				),
			};
			let track_report = tracks_map
				.entry(track.main_isrc.clone())
				.or_insert_with(|| TrackSalesReportRow {
					isrc: track.main_isrc.clone(),
					title: track.title.clone(),
					gross_royalties: BigDecimal::zero(),
				});
			track_report.gross_royalties += gross_royalties;
		}

		for (upc, gross_royalty) in sales_report.upc_map {
			let album = match project.get_album(&upc) {
				Some(album) => album,
				None => {
					println!("No album with UPC {}", upc);
					continue;
				}
			};
			let album_len = BigDecimal::from_usize(album.isrcs.len()).unwrap();
			// the .round(8) is used for a tolerance check later
			let factor = (BigDecimal::from(1) / &album_len).round(8);
			let sales_revenue_per_track: BigDecimal = &gross_royalty * factor;
			let mut remainder = gross_royalty.clone();
			for (i, isrc) in album.isrcs.clone().iter().enumerate() {
				let track = match project.get_track_by_any_isrc(&isrc) {
					Some(val) => val,
					None => panic!("No track with ISRC {}", isrc),
				};
				let track_report = tracks_map
					.entry(track.main_isrc.clone())
					.or_insert_with(|| TrackSalesReportRow {
						isrc: track.main_isrc.clone(),
						title: track.title.clone(),
						gross_royalties: BigDecimal::zero(),
					});
				let is_last = i == album.isrcs.len() - 1;
				if !is_last {
					remainder -= &sales_revenue_per_track;
					track_report.gross_royalties += &sales_revenue_per_track;
				} else {
					track_report.gross_royalties += &remainder;
					if remainder != sales_revenue_per_track {
						// tolerance is based on the .round(8)
						let diff = (&remainder - &sales_revenue_per_track).abs();
						let tolerance = BigDecimal::from_str("0.000000005").unwrap()
							* &gross_royalty * (&album_len - BigDecimal::one());
						assert!(
							diff <= tolerance,
							"Remainder diverges from the per-track amount. Code has some calculation issue",
	  				);
					}
				}
			}
		}

		TrackSalesReport {
			tracks: tracks_map,
			accounting_period_name: sales_report.accounting_period_name,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_upc_split_worst_case_rounding() {
		// 1/3 rounded to 8dp = 0.33333333, error = 0.00000000333...
		// With 3 tracks, factor = 0.33333333
		// sales_revenue_per_track = gross_royalty * 0.33333333
		// After 2 subtractions, remainder drifts by gross_royalty * 2 * 3.33e-9
		// Worst case gross_royalty: something that maximises the fractional part of gross_royalty * 0.33333333
		// The logs show gross_royalty = 2.83862... let's use that
		let gross_royalty = BigDecimal::from_str("2.83862100000000").unwrap();
		let album_len = BigDecimal::from_usize(3).unwrap();
		let factor = (BigDecimal::from(1) / &album_len).round(8);
		let sales_revenue_per_track = &gross_royalty * &factor;
		let mut remainder = gross_royalty.clone();

		for _ in 0..2 {
			remainder -= &sales_revenue_per_track;
		}

		let diff = (&remainder - &sales_revenue_per_track).abs();
		let tolerance = BigDecimal::from_str("0.000000005").unwrap()
			* &gross_royalty
			* (&album_len - BigDecimal::one());

		assert!(
			diff <= tolerance,
			"Worst case failed: diff {} exceeds tolerance {} for gross_royalty {} album_len 3",
			diff,
			tolerance,
			gross_royalty,
		);
	}
}
