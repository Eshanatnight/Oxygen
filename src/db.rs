use crate::internal_encoding::{decode_v0, encode_v0};
use color_eyre::eyre::Result;
use rusqlite::{Connection, params, types::Type};
use crate::audio_clip::AudioClip;
use chrono::prelude::*;

pub struct Db(Connection);

/*
    * For the `list` command, we can send a bunch of AudioClip entities to the console.
    * but that won't be very efficient or useful even cause we are not going to use them then and there.
    * `list` will mostly be used to load the name of the file to play.
    * so maybe sending a different struct is better, that some how will hold the name of the clips.

    * this struct just contains the metadata for the clip.
*/
pub struct ClipMeta
{
    pub clip_id: usize,
    pub clip_name: String,
    pub clip_date: DateTime<Utc>,
}

impl Db
{
    // Connection function that connects to an sqlite database file
    pub fn open() -> Result<Self>
    {
        // the sqlitefile will be stored in a directory named "data" up the src dir
        // if we consider the binary is in a directory called "bin" and
        // there is also a directory called "data"
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
                encode_v0(&clip.samples)
            ]
        )?;

        // deal with clip id
        if clip.id.is_none()
        {
            clip.id = Some(self.0.last_insert_rowid().try_into()?);
        }

        Ok(())
    }


    pub fn load(&self, name: &str) -> Result<Option<AudioClip>>
    {
        let mut stmt = self
        .0
        .prepare
        (
            "
            SELECT id, name, date, sample_rate, samples
            FROM clips
            WHERE name = ?1
            "
        )?;

        let mut clip_iter = stmt.query_map([name], |row|
            {
                let _date: String = row.get(2)?;  // we need to convert this into a `DateTime` type
                let samples: Vec<u8> = row.get(4)?;

                Ok(AudioClip
                {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    date: _date.parse().map_err(|_|
                        {
                            rusqlite::Error::InvalidColumnType(2, "date".to_string(), Type::Text)
                        })?,
                    sample_rate: row.get(3)?,
                    samples: decode_v0(&samples),
                })
            })?;

        // Basically we will check if our iterator is empty or no
        // i.e. if it has a audio clip or not, if it is there return it
        // else return None
        Ok(if let Some(clip) = clip_iter.next()
        {
            Some(clip?)
        }

        else
        {
            None
        })
    }


    // so this would retrive the information of the clips, just the basic Info
    // like name, id and date. Right now this does not take any args
    // but maybe i will add something like a filter later.
    pub fn list(&self) -> Result<Vec<ClipMeta>>
    {
        let mut stmt = self
        .0
        .prepare
        (
            "
            SELECT id, name, date
            FROM clips
            ORDER BY date
            "
        )?;

        let clip_iter = stmt.query_map([], |row|
            {
                let _date: String = row.get(2)?;  // we need to convert this into a `DateTime` type

                Ok(ClipMeta
                {
                    clip_id: row.get(0)?,
                    clip_name: row.get(1)?,
                    clip_date: _date.parse().map_err(|_|
                        {
                            rusqlite::Error::InvalidColumnType(2, "date".to_string(), Type::Text)
                        })?,
                })
            })?;

            Ok(clip_iter.collect::<Result<_, rusqlite::Error>>()?)
    }


    pub fn delete(&self, name: &str) -> Result<()>
    {

        self.0.execute(
            "
            DELETE FROM clips
            WHERE name = ?1
            ",
            params![name]
        )?;

        Ok(())
    }
}

