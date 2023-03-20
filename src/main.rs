mod client;

use anyhow::{bail, Result};
use clap::Parser;
use client::Requester;
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::ProgressBar;

fn main() -> Result<()> {
    let args = Args::parse();
    let client = Requester::new()?;
    let version = if let Some(version) = args.version {
        version
    } else {
        let spinner = ProgressBar::new_spinner();
        spinner.set_message("Getting latest Minecraft version");
        let res = client.get_latest_minecraft_version()?;
        spinner.finish_and_clear();
        res
    };
    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Getting Minecraft mods");
    let queries = client.get_queries(&args.query, &version, &args.loader)?;
    spinner.finish_and_clear();
    if queries.is_empty() {
        bail!("No projects matching Minecraft version {version} was found");
    }
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick your mod")
        .items(&queries)
        .clear(true)
        .default(0)
        .interact()?;
    let selection = &queries[selection];
    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Getting download URL");
    let link = client.get_download_url(&selection.project_id, &version, &args.loader)?;
    spinner.finish_and_clear();
    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Downloading file");
    let file = client.download_file(&link.url)?;
    spinner.finish_and_clear();
    std::fs::write(&link.filename, file)?;
    let mut dep_info = Vec::new();
    for dependency in link.dependencies {
        let spinner = ProgressBar::new_spinner();
        spinner.set_message(format!(
            "Getting dependency info for project ID: {}",
            dependency.project_id
        ));
        let name = client.get_project_name(&dependency.project_id)?;
        spinner.finish_and_clear();
        dep_info.push((name, dependency.dependency_type));
    }
    if !dep_info.is_empty() {
        println!("Mod dependencies:");
        for dep in dep_info {
            println!("{} ({})", dep.0, dep.1);
        }
        println!();
    }
    println!("Saved {} successfully!", link.filename);
    Ok(())
}

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Search query
    query: String,

    /// Minecraft version (leave this empty to fetch mods for latest Minecraft version)
    #[arg(short, long)]
    version: Option<String>,

    /// Mod loader
    #[arg(short, long, default_value = "fabric")]
    loader: String,
}
