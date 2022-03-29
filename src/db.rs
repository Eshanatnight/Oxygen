use color_eyre::eyre::Result;
use rusqlite::{Connection, params};
use crate::audio_clip::AudioClip;

pub struct Db(Connection);


fn encode(samples: &[f32]) -> Vec<u8>
{
    let mut buf = Vec::with_capacity(samples.len() * 4);

    for sample in samples
    {
        buf.extend_from_slice(&sample.to_be_bytes());
    }

    buf
}


impl Db
{
    // Connection function that connects to an sqlite database file
    pub fn open() -> Result<Self>
    {
        // the sqlitefile will be stored in a directory named "data" up the src dir
        // if we consider the binary is in a directory called "bin" and there is also a directory called "data"
        // then the sqlite file will be stored in "data/db.sqlite"
        /*
            Directory structure:
            Oxygen
            |-bin
            |-data
        */
        let connection = Connection::open("data/oxygen.sqlite")?;

        connection.pragma_update(None, "page_size", 8192)?;
        connection.pragma_update(None, "user_version", 1)?;

        connection.execute(
            "
            CREATE TABLE IF NOT EXISTS clips
            (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                date TEXT NOT NULL,
                sample_rate INTEGER NOT NULL,
                samples BLOB NOT NULL
            );
            ",
            []
        )?;

        Ok(Db(connection))
    }


    pub fn save(&self, clip: &mut AudioClip) -> Result<()>
    {
        self.0.execute(
            "
            INSERT OR REPLACE INTO clips (id, name, date, sample_rate, samples)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                clip.id,
                clip.name,
                clip.date.to_string(),
                clip.sample_rate,
                encode(&clip.samples)
            ]
        )?;

        // deal with clip id
        if clip.id.is_none()
        {
            clip.id = Some(self.0.last_insert_rowid().try_into()?);
        }

        Ok(())
    }
}