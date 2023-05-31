use crate::earnings_report::{Project, SalesReport};
use bigdecimal::{BigDecimal, FromPrimitive};
use std::collections::HashMap;

#[derive(Debug)]
pub struct TrackSalesReportRow {
	pub isrc: String,
	pub title: String,
	pub gross_royalties: BigDecimal,
}
#[derive(Debug)]
pub struct TrackSalesReport {
	pub tracks: HashMap<String, TrackSalesReportRow>,
	pub accounting_period_name: String,
}

impl TrackSalesReport {
	pub fn from_sales_report(sales_report: SalesReport, project: &Project) -> Self {
		let mut isrc_report_map = sales_report.isrc_map;

		for (upc, gross_royalty) in sales_report.upc_map {
			let album = match project.get_album(&upc) {
				Some(album) => album,
				None => {
					println!("No album with UPC {}", upc);
					continue;
				}
			};
			let album_len = BigDecimal::from_usize(album.isrcs.len()).unwrap();
			let sales_revenue_per_track = gross_royalty / album_len;
			for isrc in album.isrcs.clone() {
				*isrc_report_map.entry(isrc).or_default() += sales_revenue_per_track.clone()
			}
		}
		let tracks_map = isrc_report_map
			.into_iter()
			.map(|(isrc, gross_royalties)| {
				let track = match project.get_track_by_any_isrc(&isrc) {
					Some(val) => val,
					None => panic!("No track with ISRC {}", isrc),
				};
				let row = TrackSalesReportRow {
					isrc: track.main_isrc.clone(),
					title: track.title.clone(),
					gross_royalties,
				};
				(isrc, row)
			})
			.collect();
		TrackSalesReport {
			tracks: tracks_map,
			accounting_period_name: sales_report.accounting_period_name,
		}
	}
}
