use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use earnings_report::Project;
use std::path::PathBuf;

mod earnings_report;
mod project_data;
mod settings;
mod sources;
mod track_sales_report;
mod transformers;

#[derive(Parser)]
#[command(about, long_about)]
struct Cli {
	/// Path to .jsonc settings file
	settings_path: PathBuf,
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Generate results for an accounting period
	Generate(Generate),
	/// Export already-generated accounting results
	Export(Export),
}

#[derive(Args)]
struct Generate {
	/// Accounting period to generate results for
	accounting_period_name: String,
	/// Save accounting period results
	#[arg(long)]
	save: bool,
}

#[derive(Args)]
struct Export {
	/// Accounting period to generate results for
	accounting_period_name: String,
}

fn main() -> Result<()> {
	let args = Cli::parse();

	let mut project = Project::load(PathBuf::from(args.settings_path))?;
	project.verify()?;

	match args.command {
		Commands::Generate(args) => {
			let accounting_period = project
				.get_accounting_period(&args.accounting_period_name)
				.context("Accounting period from argument not found")?;
			let accounting_result = accounting_period.generate_result(&project)?;

			if args.save {
				project.add_and_save_result(accounting_result)?;
			} else {
				println!("Finished! Re-run with --save it all seems good");
			}
		}

		Commands::Export(args) => {
			let result = project
				.data
				.get_accounting_result(&args.accounting_period_name)
				.context("No result")?;
			let file_name = result.name.to_string() + ".json";
			let export = result.export();
			export.save_to_downloads(file_name)?;
			return Ok(());
		}
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
