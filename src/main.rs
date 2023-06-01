use anyhow::{Context, Result};
use earnings_report::Project;
use std::env;
use std::path::PathBuf;

mod earnings_report;
mod project_data;
mod settings;
mod sources;
mod track_sales_report;
mod transformers;

fn main() -> Result<()> {
	let settings_path = env::args()
		.nth(1)
		.context("Usage: <settings_path> <accounting_period_name>")?;
	let accounting_period_name = env::args()
		.nth(2)
		.context("Usage: <settings_path> <accounting_period_name>")?;

	let mut project = Project::load(PathBuf::from(settings_path))?;
	project.verify()?;

	let accounting_period = project
		.get_accounting_period(&accounting_period_name)
		.context("Accounting period from argument not found")?;
	let accounting_result = accounting_period.generate_result(&project)?;

	project.save_result(accounting_result)?;

	Ok(())
}
