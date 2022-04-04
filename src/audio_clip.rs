use chrono::prelude::*;
use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, Sample};
use color_eyre::eyre::{Result, eyre};
use dasp::{interpolate::linear::Linear, signal, Signal};
use std::sync::{Arc, Mutex, mpsc::{Sender, channel}};

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
    pub fn new(sample_rate: u32, samples: Vec<f32>, id: Option<usize>, name: String) -> Self
    {
        Self {
            samples,
            sample_rate,
            id,
            name,
            date: Utc::now(),
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
            T: Sample,
        {
            if let Ok(mut guard) = writer.try_lock()
            {
                if let Some(clip) = guard.as_mut()
                {
                    for frame in input.chunks(channels.into())
                    {
                        clip.samples.push(frame[0].to_f32());
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
            )?,

            cpal::SampleFormat::I16 => device.build_input_stream
            (
                &config.into(),
                move |data, _: &_| write_input_data::<i16>(data, channels, &clip_2),
                err_fn,
            )?,

            cpal::SampleFormat::U16 => device.build_input_stream
            (
                &config.into(),
                move |data, _: &_| write_input_data::<u16>(data, channels, &clip_2),
                err_fn,
            )?,
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
            T: Sample,
        {
            if let Ok(mut guard) = writer.try_lock()
            {
                if let Some((i, clip_samples, done)) = guard.as_mut()
                {
                    for frame in output.chunks_mut(channels.into())
                    {
                        for sample in frame.iter_mut()
                        {
                            *sample = Sample::from(clip_samples.get(*i).unwrap_or(&0f32));
                        }
                        *i+=1;
                    }

                    if *i >= clip_samples.len()
                    {
                        if let Err(_) = done.send(())
                        {
                            // Playback has already ended. We will be dead soon.
                        }
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
            )?,

            cpal::SampleFormat::I16 => device.build_output_stream
            (
                &config.into(),
                move |data, _: &_| write_output_data::<i16>(data, channels, &state),
                err_fn,
            )?,

            cpal::SampleFormat::U16 => device.build_output_stream
            (
                &config.into(),
                move |data, _: &_| write_output_data::<u16>(data, channels, &state),
                err_fn,
            )?,
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
}