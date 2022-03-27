#[allow(non_snake_case)]
mod audio_clip;
use color_eyre::eyre::Result;
use clap::{Parser, Subcommand};
use audio_clip::AudioClip;

#[derive(Debug, Parser)]
#[clap(name = "Oxygen")]
#[clap(about = "Voice Journal Tool", long_about = None)]
struct Cli
{
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands
{
    /// Record the voice clip with the default input device untill `ctrl+c` is pressed
    Record
    {
        /// name of the audio clip to record, if not specified, the current date and time will be used
        name: Option<String>,
    },

    /// List all the audio clips in the database
    List
    {},

    /// play the clip with the specified name
    #[clap(arg_required_else_help = true)]
    Play
    {
        /// Name of the audio clip to play
        name: String,
    },

    /// delete the clip with the specified name
    #[clap(arg_required_else_help = true)]
    Delete
    {
        /// Name of the audio clip to delete
        name: String,
    },

}


fn main() -> Result<()>
{
    color_eyre::install()?;
    let args = Cli::parse();

    match args.command
    {
        Commands::Record {name } =>
        {
            let name = name.unwrap_or_else(|| "untitled".to_string());
            let clip = AudioClip::record()?;
            clip.play()?;
            todo!()
        }

        Commands::List {}=>
        {
            todo!()
        }

        Commands::Play{name} =>
        {
            eprintln!("Play: {}", name);
            todo!()
        }

        Commands::Delete{name} =>
        {
            eprintln!("Delete: {}", name);
            todo!();
        }
    }
}
