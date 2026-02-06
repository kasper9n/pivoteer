use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use project::Project;
use std::path::PathBuf;

mod accounting;
mod generator;
mod manifest;
mod manifest_old;
mod project;
mod project_data;
mod sources;
mod track_sales_report;
mod transformers;

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

fn main() -> Result<()> {
	let args = Cli::parse();
	let manifest_path = args.manifest_path;

	match args.command {
		Commands::Migrate(arg) => {
			let settings = manifest_old::Settings::from_path(manifest_path);
			let manifest = settings.migrate();
			let buf = to_json_string_pretty(&manifest)?;
			std::fs::write(arg.destination_path, buf).context("Failed to write data file")?;
			return Ok(());
		}
		_ => {}
	}

	let mut project = Project::load(PathBuf::from(manifest_path))?;
	project.validate()?;

	match args.command {
		Commands::Migrate(_) => {}

		Commands::Generate(args) => {
			generator::generate_all_open(&mut project)?;
			// let accounting_period = project
			// 	.get_accounting_period(&args.accounting_period_name)
			// 	.context("Accounting period from argument not found")?;
			// let accounting_result = accounting_period.generate_result(&project)?;

			// if args.save {
			// 	project.add_and_save_result(accounting_result)?;
			// } else {
			// 	println!("Finished! Re-run with --save it all seems good");
			// }
		}

		Commands::GenerateUpTo(args) => {
			// let periods = project.accounting_periods.clone();
			// for accounting_period in periods {
			// 	let end_now = accounting_period.name == args.accounting_period_name;
			// 	let accounting_period = project
			// 		.get_accounting_period(&accounting_period.name)
			// 		.context("Accounting period from argument not found")?;
			// 	let accounting_result = accounting_period.generate_result(&project)?;

			// 	project.add_result(accounting_result)?;
			// 	if args.save {
			// 		project.accounting.save(&project.data_file_path)?;
			// 	} else {
			// 		println!("Finished! Re-run with --save it all seems good");
			// 	}

			// 	if end_now {
			// 		break;
			// 	}
			// }
		}

		Commands::Export(args) => {
			// let result = project
			// 	.accounting
			// 	.get_accounting_result(&args.accounting_period_name)
			// 	.context("No result")?;
			// let file_name = result.name.to_string() + ".json";
			// let export = result.export();
			// export.save_to_downloads(file_name)?;
			// return Ok(());
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
