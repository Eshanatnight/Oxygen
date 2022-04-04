extern crate core;

mod audio_clip;
mod db;
mod internal_encoding;

use audio_clip::AudioClip;
use chrono::prelude::*;
use clap::{Parser, Subcommand};
use color_eyre::{eyre::eyre, Result};
use db::Db;

#[derive(Debug, Parser)]
#[clap(name = "Oxygen")]
#[clap(about = "Voice Journal Tool", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Record the voice clip with the default input device untill `ctrl+c` is pressed
    Record {
        /// name of the audio clip to record, if not specified, the current date and time will be used
        name: Option<String>,
    },

    /// List all the audio clips in the database
    List {},

    /// play the clip with the specified name
    #[clap(arg_required_else_help = true)]
    Play {
        /// Name of the audio clip to play
        name: String,
    },

    /// delete the clip with the specified name
    #[clap(arg_required_else_help = true)]
    Delete {
        /// Name of the audio clip to delete
        name: String,
    },

    /// Takes a path and a name and imports the file to the database
    #[clap(arg_required_else_help = true)]
    Import{
        /// Name of the path as a unicode string
        path: String,
        /// name of the file to import
        name: Option<String>
    },
}

fn main() -> Result<()>
{
    color_eyre::install()?;
    let args = Cli::parse();
    let db = Db::open()?;

    match args.command
    {
        Commands::Record { name } =>
        {
            let name = name.unwrap_or_else(|| Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());

            if db.load(&name)?.is_some()
            {
                return Err(eyre!("Clip with this name already exists. Please rename the clip"));
            }

            let mut clip = AudioClip::record(name)?;

            db.save(&mut clip)?;

        }

        Commands::List {} =>
        {

            println!("{id:>5}  {name:30} {date:30}" , id="ID", name="Name", date="Date");

            for entry in db.list()?
            {
                // ? the DateTime struct will print the date and time in the format
                // ? "%Y-%m-%d %H:%M:%S"
                println!(
                    "{:5}  {:30} {:30}",
                    entry.clip_id,
                    entry.clip_name,
                    entry
                        .clip_date
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                );
            }
        }

        Commands::Play { name } =>
        {
            if let Some(clip) = db.load(&name)?
            {
                clip.play()?;
            }

            else
            {
                eprintln!("No clip with the name {} found", name);
            }
        }

        Commands::Delete { name } =>
        {
            db.delete(&name)?;
        }

        Commands::Import{path, name} =>
        {
            let name = name.unwrap_or_else(|| path.clone());

            if db.load(&name)?.is_some()
            {
                return Err(eyre!("Clip with this name already exists. Please rename the file"));
            }

            let mut clip = AudioClip::import(name, path)?;
            db.save(&mut clip)?;
        }
    }

    Ok(())
}
