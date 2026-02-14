use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use project::Project;
use std::path::PathBuf;

use crate::{project::YearQuarter, project_data::save_to_downloads};

mod accounting;
mod generator;
mod manifest;
mod project;
mod project_data;
mod sources;
mod track_sales_report;

#[derive(Parser)]
#[command(about, long_about)]
struct Cli {
	/// Path to .jsonc manifest file
	manifest_path: PathBuf,
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Generate results for an accounting period
	Migrate(Migrate),
	/// Generate results for an accounting period
	Generate(Generate),
	/// Generate results for all accounting periods up to the specified one
	GenerateUpTo(GenerateUpTo),
	/// Export already-generated accounting results
	Export(Export),
	/// Export already-generated accounting results for all accounting periods up
	/// to the specified one
	ExportUpTo(ExportUpTo),
}

#[derive(Args)]
struct Migrate {
	/// Path to .jsonc manifest file
	destination_path: PathBuf,
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
struct GenerateUpTo {
	/// Accounting period to generate results up to
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

#[derive(Args)]
struct ExportUpTo {
	/// Accounting period to generate results for
	accounting_period_name: String,
}

fn main() -> Result<()> {
	let args = Cli::parse();
	let manifest_path = args.manifest_path;

	let mut project = Project::load(PathBuf::from(manifest_path))?;
	project.validate()?;

	match args.command {
		Commands::Migrate(_) => {}

		Commands::Generate(args) => {
			let pname = YearQuarter::parse(&args.accounting_period_name);
			let result = generator::generate(&project, &pname)?;

			if args.save {
				project.add_and_save_result(result)?;
			} else {
				println!("Finished! Re-run with --save it all seems good");
			}
		}

		Commands::GenerateUpTo(args) => {
			let pname = YearQuarter::parse(&args.accounting_period_name);

			let periods = project.accounting_periods.clone();
			for accounting_period in periods {
				if accounting_period.prev_period() == pname {
					break;
				}

				let result = generator::generate(&project, &accounting_period.name)?;

				project.add_result(result)?;
				if args.save {
					project.data.save(&project.data_file_path)?;
				} else {
					println!("Finished! Re-run with --save it all seems good");
				}
			}
		}

		Commands::Export(args) => {
			let pname = YearQuarter::parse(&args.accounting_period_name);

			let result = project
				.data
				.get_accounting_result(&pname)
				.context("No result")?;
			let file_name = result.name.to_string() + ".json";
			let export = result.export(&project);
			save_to_downloads(&vec![export], file_name)?;
		}

		Commands::ExportUpTo(args) => {
			let pname = YearQuarter::parse(&args.accounting_period_name);

			let periods = project.accounting_periods.clone();
			let first_name = periods.first().unwrap().name.clone();
			let mut last_name = first_name.clone();
			let mut exports = Vec::new();
			for accounting_period in &periods {
				if accounting_period.prev_period() == pname {
					break;
				}
				last_name = accounting_period.name.clone();

				let result = project
					.data
					.get_accounting_result(&accounting_period.name)
					.context("No result")?;
				let export = result.export(&project);
				exports.push(export);
			}

			let file_name = format!(
				"{} - {}.json",
				first_name.to_string(),
				last_name.to_string()
			);
			save_to_downloads(&exports, file_name)?;
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
