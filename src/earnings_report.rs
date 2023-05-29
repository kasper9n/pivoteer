use crate::settings::Settings;
use crate::sources::Source;
use bigdecimal::BigDecimal;
use csv_pipeline::target::StdoutTarget;
use csv_pipeline::{Pipeline, Transformer};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::env;
use std::path::PathBuf;

pub struct Project {
	accounting_periods: Vec<AccountingPeriod>,
}
impl Project {
	fn new(dir: PathBuf, settings: Settings) -> Self {
		let accounting_periods = settings
			.accounting_periods
			.into_iter()
			.map(|accounting_period| {
				let sources = accounting_period.to_sources(&dir);
				AccountingPeriod {
					name: accounting_period.name,
					sources,
				}
			})
			.collect();

		Project { accounting_periods }
	}
	pub fn load() -> Self {
		let settings_path = match env::args().nth(1) {
			Some(arg) => PathBuf::from(arg),
			None => {
				panic!("No Sources.toml argument given");
			}
		};
		let project_dir = settings_path.parent().unwrap().to_owned();
		let settings = Settings::from_path(settings_path);
		Self::new(project_dir, settings)
	}
	pub fn get_all_sources(&self) -> Vec<Source> {
		self.accounting_periods
			.iter()
			.map(|accounting_period| accounting_period.sources.clone())
			.flatten()
			.collect()
	}
}

struct AccountingPeriod {
	name: String,
	sources: Vec<Source>,
}
impl AccountingPeriod {
	fn generate(&self) {
		let files: Vec<_> = self
			.sources
			.par_iter()
			.map(|source| {
				into_earnings_report(source.process_source())
					.collect_into_rows()
					.unwrap()
			})
			.collect();
		let pipelines = files.into_iter().map(|rows| {
			return Pipeline::from_rows(rows).unwrap();
		});
		let funnel_pipeline = Pipeline::from_pipelines(pipelines);
		into_earnings_report(funnel_pipeline)
			// .flush(PathTarget::new(format!("reports/{}.csv", self.name)))
			.flush(StdoutTarget::new())
			.run()
			.unwrap();
	}
}

pub fn into_earnings_report(pipeline: Pipeline) -> Pipeline {
	pipeline.transform_into(|| {
		vec![
			Transformer::new("Gross Royalties").sum(BigDecimal::from(0)),
			Transformer::new("ISRC").keep_unique(),
			Transformer::new("UPC").keep_unique(),
		]
	})
}

pub fn generate_all() {
	let project = Project::load();
	let accounting_periods = project.accounting_periods;

	accounting_periods.par_iter().for_each(|accounting_period| {
		accounting_period.generate();
	});

	// for accounting_period in project.accounting_periods {
	// 	let files: Vec<_> = accounting_period
	// 		.sources
	// 		.par_iter()
	// 		.map(|source| {
	// 			into_earnings_report(source.process_source())
	// 				.collect_into_rows()
	// 				.unwrap()
	// 		})
	// 		.collect();
	// 	let pipelines = files.into_iter().map(|rows| {
	// 		return Pipeline::from_rows(rows).unwrap();
	// 	});
	// }

	// let funnel_pipeline = Pipeline::from_pipelines(pipelines);
	// into_earnings_report(funnel_pipeline)
	// 	.flush(PathTarget::new("reports/Full.csv"))
	// 	.run()
	// 	.unwrap();
}
