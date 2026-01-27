use crate::project::{Project, SalesReport};
use bigdecimal::{BigDecimal, FromPrimitive, Zero};
use std::collections::HashMap;

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
	pub accounting_period_name: String,
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
					unmapped_isrc, sales_report.accounting_period_name
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
			let factor = (BigDecimal::from(1) / album_len).round(8);
			let sales_revenue_per_track: BigDecimal = gross_royalty * factor;
			for isrc in album.isrcs.clone() {
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
				track_report.gross_royalties += sales_revenue_per_track.clone()
			}
		}

		TrackSalesReport {
			tracks: tracks_map,
			accounting_period_name: sales_report.accounting_period_name,
		}
	}
}
