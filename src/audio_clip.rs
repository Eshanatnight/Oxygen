#![allow(non_snake_case)]
use chrono::prelude::*;
use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, Sample};
use color_eyre::eyre::{Result, eyre};
use dasp::{interpolate::linear::Linear, signal, Signal};
use std::{sync::{Arc, Mutex, mpsc::{Sender, channel}}};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use std::fs::File;
use std::path::Path;

use hound;

///Raw Mono Audio Data
#[derive(Clone)]
pub struct AudioClip
{
    pub samples: Vec<f32>,
    pub sample_rate: u32,   // Most Common -> 48kHz : 44.1kHz

    // decided to save the meta data `date-time` to be stored here
    pub id: Option<usize>,
    pub name: String,
    pub date: DateTime<Utc>,
}

type ClipHandle = Arc<Mutex<Option<AudioClip>>>;

impl AudioClip
{
    #[allow(dead_code)]
    pub fn new(sample_rate: u32, samples: Vec<f32>, id: Option<usize>, name: String, date: DateTime<Utc>) -> Self
    {
        Self {
            samples,
            sample_rate,
            id,
            name,
            date,
        }
    }

    pub fn record(_name: String) -> Result<AudioClip>
    {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| eyre!("No input device found"))?;

        println!("Input Device: {}", device.name()?);

        let config = device.default_input_config()?;

        let clip = AudioClip {
            id: None,
            date: Utc::now(),
            samples: Vec::new(),
            name: _name,
            sample_rate: config.sample_rate().0,
        };

        println!("Begin Recording...");

        let clip = Arc::new(Mutex::new(Some(clip)));
        let clip_2 = clip.clone();

        let err_fn = move |err|
        {
            eprintln!("an error occurred on stream: {}", err);
        };

        let channels = config.channels();
        /// We are just going to focus on mono channel data.
        /// We can go with panoramic audio if we want to, but for a Voice Journal is it Necessary?

        fn write_input_data<T>(input: &[T], channels: u16, writer: &ClipHandle)
        where
            T: Sample, f32: cpal::FromSample<T>,
        {
            if let Ok(mut guard) = writer.try_lock()
            {
                if let Some(clip) = guard.as_mut()
                {
                    for frame in input.chunks(channels.into())
                    {
                        clip.samples.push(frame[0].to_sample::<f32>());
                    }
                }
            }
        }

        let stream = match config.sample_format()
        {
            cpal::SampleFormat::F32 => device.build_input_stream
            (
                &config.into(),
                move |data, _: &_| write_input_data::<f32>(data, channels, &clip_2),
                err_fn,
                None,
            )?,

            cpal::SampleFormat::I16 => device.build_input_stream
            (
                &config.into(),
                move |data, _: &_| write_input_data::<i16>(data, channels, &clip_2),
                err_fn,
                None,
            )?,

            cpal::SampleFormat::U16 => device.build_input_stream
            (
                &config.into(),
                move |data, _: &_| write_input_data::<u16>(data, channels, &clip_2),
                err_fn,
                None,
            )?,
            _ => return Err(eyre!("Unsupported Sample Format")), // maybe deal with this somewhen
        };

        stream.play()?;

        let (tx, rx) = channel();
        ctrlc::set_handler(move || tx.send(()).expect("Could not send signal over the chanel..."))?;
        println!("Press Ctrl-C to stop recording...");
        rx.recv()?;
        println!("Got it! Stopping recording...");

        drop(stream);
        let clip = clip.lock().unwrap().take().unwrap();


        eprintln!("Recorded {} samples", clip.samples.len());
        Ok(clip)
    }


    pub fn play(&self) -> Result<()>
    {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| eyre!("No output device found"))?;

        println!("Output Device: {}", device.name()?);

        let config = device.default_output_config()?;

        println!("Beginning Playback...");

        type StateHandle = Arc<Mutex<Option<(usize, Vec<f32>, Sender<()>)>>>;
        let sample_rate = config.sample_rate().0;
        let (done_tx, done_rx) = channel::<()>();
        let state = (0, self.resample(sample_rate).samples, done_tx);
        let state = Arc::new(Mutex::new(Some(state)));
        let channels = config.channels();


        let err_fn = move |err|
        {
            eprintln!("an error occurred on stream: {}", err);
        };


        fn write_output_data<T>(output: &mut [T], channels: u16, writer: &StateHandle)
        where
            T: Sample + cpal::FromSample<f32>,
        {
            if let Ok(mut guard) = writer.try_lock()
            {
                if let Some((i, clip_samples, done)) = guard.as_mut()
                {
                    for frame in output.chunks_mut(channels.into())
                    {
                        for sample in frame.iter_mut()
                        {
                            *sample = Sample::from_sample(clip_samples.get(*i).unwrap_or(&0f32).to_owned());
                        }
                        *i+=1;
                    }

                    if *i >= clip_samples.len() && done.send(()).is_err()
                    {
                        // Playback has already ended. We will be dead soon.
                    }
                }
            }
        }

        let stream = match config.sample_format()
        {
            cpal::SampleFormat::F32 => device.build_output_stream
            (
                &config.into(),
                move |data, _: &_| write_output_data::<f32>(data, channels, &state),
                err_fn,
                None,
            )?,

            cpal::SampleFormat::I16 => device.build_output_stream
            (
                &config.into(),
                move |data, _: &_| write_output_data::<i16>(data, channels, &state),
                err_fn,
                None,
            )?,

            cpal::SampleFormat::U16 => device.build_output_stream
            (
                &config.into(),
                move |data, _: &_| write_output_data::<u16>(data, channels, &state),
                err_fn,
                None,
            )?,

            _ => return Err(eyre!("Unsupported Sample Format")), // maybe deal with this somewhen
        };
        stream.play()?;

        done_rx.recv()?;
        Ok(())
    }


    pub fn resample(&self, sample_rate: u32) -> AudioClip
    {
        if self.sample_rate == sample_rate
        {
            return self.clone();
        }

        let mut signal = signal::from_iter(self.samples.iter().copied());
        let a = signal.next();
        let b = signal.next();

        let linear = Linear::new(a, b);

        AudioClip
        {
            id: self.id,
            name: self.name.clone(),
            date: self.date,
            samples: signal.from_hz_to_hz(linear, self.sample_rate as f64, sample_rate as f64)
                        .take(self.samples.len() * (sample_rate as usize) / self.sample_rate as usize)
                        .collect(),

            sample_rate,
        }
    }


    pub fn import(name: String, path: String) -> Result<AudioClip>
    {
        // Create a media source.
        // MediaSource trait is automatically implemented for File
        let file = Box::new(File::open(Path::new(&path))?);

        let creation_time = DateTime::<Utc>::from(file.metadata()?.created()?);

        // Create the media source stream using the boxed media source from above.
        let mss = MediaSourceStream::new(file, Default::default());

        // Create a hint to help the format registry guess what format reader is appropriate.
        let hint = Hint::new();

        // Use the default options when reading and decoding.
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        // Probe the media source stream for a format.
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)?;

        // Get the format reader yielded by the probe operation.
        let mut format = probed.format;

        // Get the default track.
        let track = format
            .default_track()
            .ok_or_else(|| eyre!("No default track"))?;

        // Create a decoder for the track.
        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts).unwrap();

        // Store the track identifier, we'll use it to filter packets.
        let track_id = track.id;

        let mut sample_count = 0;
        let mut sample_buf = None;
        let channels = track.codec_params.channels.ok_or_else(|| eyre!("Unknown Number of Channels"))?;

        let mut clip = AudioClip::new(
            track
            .codec_params.
            sample_rate.
            ok_or_else(|| eyre!("Unknown Sample Rate"))?,
            Vec::new(),
            None,
            name,
            creation_time,
        );

        loop
        {
            // Get the next packet from the format reader.
            // This is hacky, but it works!
            let packet = match format.next_packet()
            {
                Ok(packet_ok) => packet_ok,

                Err(Error::IoError(ref packet_err))
                    if packet_err.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        break;
                    },

                Err(packet_err) =>
                {
                    return Err(packet_err.into());
                },
            };

            // If the packet does not belong to the selected track, skip it.
            if packet.track_id() != track_id
            {
                continue;
            }

            // Decode the packet into audio samples, ignoring any decode errors.
            match decoder.decode(&packet)
            {
                Ok(audio_buf) =>
                {
                    // The decoded audio samples may now be accessed via the audio buffer if per-channel
                    // slices of samples in their native decoded format is desired. Use-cases where
                    // the samples need to be accessed in an interleaved order or converted into
                    // another sample format, or a byte buffer is required, are covered by copying the
                    // audio buffer into a sample buffer or raw sample buffer, respectively. In the
                    // example below, we will copy the audio buffer into a sample buffer in an
                    // interleaved order while also converting to a f32 sample format.

                    // If this is the *first* decoded packet, create a sample buffer matching the
                    // decoded audio buffer format.
                    if sample_buf.is_none()
                    {
                        // Get the audio buffer specification.
                        let spec = *audio_buf.spec();

                        // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                        let duration = audio_buf.capacity() as u64;

                        // Create the f32 sample buffer.
                        sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                    }

                    // Copy the decoded audio buffer into the sample buffer in an interleaved format.
                    if let Some(buf) = &mut sample_buf
                    {
                        buf.copy_interleaved_ref(audio_buf);
                        let mono: Vec<f32> = buf.samples().iter().step_by(channels.count()).copied().collect();

                        clip.samples.extend_from_slice(&mono);

                        // The samples may now be access via the `samples()` function.
                        sample_count += buf.samples().len();
                        print!("\rDecoded {} samples", sample_count);
                    }
                }

                Err(Error::DecodeError(_)) => (),

                Err(_) => break,
            }
        }

        Ok(clip)
    }


    pub fn export(&self, path: &str) -> Result<()>
    {
        if !path.ends_with(".wav")
        {
            return Err(eyre!("Expected the path to end with `.wav`.\nPath given : {}", path));
        }
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;

        for sample in &self.samples
        {
            writer.write_sample(*sample)?;
        }

        writer.finalize()?;

        Ok(())
    }
}