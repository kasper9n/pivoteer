use earnings_report::Project;
use rayon::prelude::*;

mod earnings_report;
mod settings;
mod sources;
mod transformers;

fn main() {
	let project = Project::load();
	let track_sales_report = project
		.accounting_periods
		.par_iter()
		.map(|accounting_period| {
			let sales_report = accounting_period.generate_sales_report();
			let track_sales_report = sales_report.into_track_sales_report(&project);
			track_sales_report
		})
		.collect::<Vec<_>>();
}
