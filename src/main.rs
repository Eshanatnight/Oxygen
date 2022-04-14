#![allow(non_snake_case)]
extern crate core;

mod audio_clip;
mod db;
mod internal_encoding;

use std::ffi::OsStr;

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

    /// play the last recorded clip
    PlayLast{},

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

    /// Export the clip with the specified name to the specified path
    /// as a wav file
    #[clap(arg_required_else_help = true)]
    Export
    {
        /// Name of the audio clip to export
        name: String,
        /// Name of the path as a unicode string
        path: String
    },

    /// Exports all the clips in the database to the specified path
    /// of the folder to export the wav files
    #[clap(arg_required_else_help = true)]
    ExportAll
    {
        folder: String
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
                return Err(eyre!("No clip with the name {} found", name));
            }
        }

        Commands::PlayLast{} =>
        {
            if let Some(clip) = db.load_last()?
            {
                println!("Playing Last Clip");
                clip.play()?;
            }
            else
            {
                return Err(eyre!("No Clip found Empty Database"));
            }
        }


        Commands::Delete { name } =>
        {
            db.delete(&name)?;
        }

        Commands::Import{path, name} =>
        {
            let name = match name
            {
                Some(name) => name,

                None =>
                {
                    std::path::Path::new(&path)
                    .file_stem()
                    .ok_or_else(|| eyre!("Invalid Path"))?
                    .to_str()
                    .ok_or_else(|| eyre!("Invalid Path not utf8"))?
                    .to_string()
                }

            };

            if db.load(&name)?.is_some()
            {
                return Err(eyre!("Clip with this name already exists. Please rename the file"));
            }

            let mut clip = AudioClip::import(name, path)?;
            db.save(&mut clip)?;
        }

        Commands::Export { name, path } =>
        {
            if let Some(clip) = db.load(&name)?
            {
                clip.export(&path)?;
            }

            else
            {
                return Err(eyre!("No clip with the name {} found", name));
            }
        }

        Commands::ExportAll { folder } =>
        {
            let path = std::path::Path::new(&folder);

            if !path.exists()
            {
                println!("Creating folder {}", folder);

                std::fs::create_dir(path)?;
            }

            let mut children = path.read_dir()?;

            if children.next().is_some()
            {
                return Err(eyre!("Folder {} is not empty.\nExpected an empty directory", folder));
            }

            for entry in db.list()?
            {
                if let Some(clip) = db.load(&entry.clip_name)?
                {
                    let safe_name = std::path::Path::new(&entry.clip_name)
                    .file_name()
                    .unwrap_or_else( || OsStr::new("invalid"))
                    .to_str()
                    .ok_or_else(|| eyre!("Invalid path.\nNot valid utf8"))?
                    .to_string();

                    let export_path = path.join(
                        std::path::Path::new(&format!("{}_{}.wav",safe_name, entry.clip_id)));

                    let export_path = export_path.as_path()
                        .to_str()
                        .ok_or_else(|| eyre!("Invalid path.\nNot valid utf8"))?;

                        clip.export(&export_path)?;
                }

                else
                {
                    return Err(eyre!("{} clip was removed during export", entry.clip_name));
                }
            }
        }
    }

    Ok(())
}
