use anyhow::{Context, Result};
use clap::Parser;
use earnings_report::Project;
use std::path::PathBuf;

mod earnings_report;
mod project_data;
mod settings;
mod sources;
mod track_sales_report;
mod transformers;

#[derive(Parser)]
#[command(about, long_about = None)]
struct Args {
	/// Path to .jsonc settings file
	settings_path: PathBuf,

	/// Accounting period to generate results for
	accounting_period_name: String,

	// Save accounting period results
	#[arg(long)]
	save: bool,
}

fn main() -> Result<()> {
	let args = Args::parse();

	let mut project = Project::load(PathBuf::from(args.settings_path))?;
	project.verify()?;

	let accounting_period = project
		.get_accounting_period(&args.accounting_period_name)
		.context("Accounting period from argument not found")?;
	let accounting_result = accounting_period.generate_result(&project)?;

	if args.save {
		project.save_result(accounting_result)?;
	}

	Ok(())
}
