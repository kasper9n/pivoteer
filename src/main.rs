use anyhow::{Context, Result};
use clap::Parser;
use earnings_report::Project;
use std::fs::OpenOptions;
use std::io::Write;
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

	// Export result to Kyper
	#[arg(long)]
	export: bool,
}

fn main() -> Result<()> {
	let args = Args::parse();

	let mut project = Project::load(PathBuf::from(args.settings_path))?;
	project.verify()?;

	if args.export {
		let result = project
			.data
			.get_accounting_result(&args.accounting_period_name)
			.context("No result")?;
		let export = result.export();
		let buf = to_json_string_pretty(&export)?;
		let file_path = dirs_next::download_dir()
			.context("Failed to get download dir")?
			.join(result.name.to_string() + ".json");
		let mut file = OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&file_path)?;
		file.write_all(&buf)?;

		return Ok(());
	}
	let accounting_period = project
		.get_accounting_period(&args.accounting_period_name)
		.context("Accounting period from argument not found")?;
	let accounting_result = accounting_period.generate_result(&project)?;

	if args.save {
		project.add_and_save_result(accounting_result)?;
	}

	Ok(())
}

pub fn to_json_string_pretty(value: &impl serde::Serialize) -> Result<Vec<u8>> {
	let mut buf = Vec::new();
	let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
	let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
	value
		.serialize(&mut ser)
		.context("Failed to serialize data")?;
	Ok(buf)
}
