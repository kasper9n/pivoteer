use bigdecimal::BigDecimal;
use csv_pipeline::target::PathTarget;
use csv_pipeline::{Pipeline, Transformer};
use earnings_report::Project;

use crate::transformers::SumOrKeepTransform;

mod earnings_report;
mod settings;
mod sources;
mod transformers;

fn main() {
	earnings_report::generate_all();
	// artist_statement();
}

pub fn artist_statement() {
	// let artist_name = "RYLLZ";
	// let artist_isrcs = vec![
	// 	"USLZJ1888276", // nemesis
	// 	"CALVP1903197", // purgatory
	// 	"CALVP2062669", // excalibur
	// 	"CAHQJ2117718", // warrior
	// 	"CAENV2262507", // nemesis II
	// ];

	// let artist_name = "M.I.M.E";
	// let artist_isrcs = vec![
	// 	"CACWV2063398", // M.I.M.E, Drama B & BULGANG - Slide
	// ];

	// let artist_name = "Jey Vazz";
	// let artist_isrcs = vec![
	// 	"QZ5FN1852162", // Jey Vazz - Crazy For You (feat. Vikki Gilmore)
	// ];

	let artist_name = "Frizzy The Streetz";
	let artist_isrcs = vec![
		"CAHQJ2153230", // Part Of & Frizzy The Streetz - Losing Myself
		"CAENV2119873", // Kaphy x Frizzy The Streetz x SØR - Doubt
		"CACWV2200649", // DNIE & Frizzy The Streetz - Dream
		"CAHQJ2253671", // DNIE & Frizzy The Streetz - Dream (Yonexx Remix)
	];

	let sources = Project::load().get_all_sources();
	let pipelines = sources.iter().map(|source| {
		source.process_source().filter(|headers, row| {
			let isrc = headers.get_field(&row, "ISRC").unwrap();
			artist_isrcs.contains(&isrc)
		})
	});

	Pipeline::from_pipelines(pipelines)
		.transform_into(|| {
			vec![
				Transformer::new("Reporting Period").keep_unique(),
				Transformer::new("UPC").keep_unique(),
				Transformer::new("ISRC").keep_unique(),
				Transformer::new("Store").keep_unique(),
				Transformer::new("Store service").keep_unique(),
				Transformer::new("Units").sum_but_keep(0 as i64, ""),
				Transformer::new("Gross Royalties").sum(BigDecimal::from(0)),
			]
		})
		.flush(PathTarget::new(format!("reports/{artist_name} Report.csv")))
		.run()
		.unwrap();
}
