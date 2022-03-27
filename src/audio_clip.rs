use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, Sample};
use color_eyre::eyre::{Result, eyre};
use std::sync::{Arc, Mutex};

///Raw Mono Audio Data
#[derive(Clone)]
pub struct AudioClip
{
    samples: Vec<f32>,
    sample_rate: u32,   // Most Common -> 48kHz : 44.1kHz
}

type ClipHandle = Arc<Mutex<Option<AudioClip>>>;

impl AudioClip
{
    pub fn record() -> Result<AudioClip>
    {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| eyre!("No input device found"))?;

        println!("Input Device: {}", device.name()?);

        let config = device.default_input_config()?;

        let clip = AudioClip {
            samples: Vec::new(),
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

        // Let recording go for roughly three seconds.
        std::thread::sleep(std::time::Duration::from_secs(3));
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

        println!("Begining Playback...");

        type StateHandle = Arc<Mutex<Option<(usize, Vec<f32>)>>>;
        let sample_rate = config.sample_rate().0;
        let state = (0, self.resample(sample_rate).samples);
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
                if let Some((i, clip_samples)) = guard.as_mut()
                {
                    for frame in output.chunks_mut(channels.into())
                    {
                        for sample in frame.iter_mut()
                        {
                            *sample = Sample::from(clip_samples.get(*i).unwrap_or(&0f32));
                        }

                        *i+=1;
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

        // Let recording go for roughly three seconds.
        std::thread::sleep(std::time::Duration::from_secs(3));


        Ok(())
    }


    pub fn resample(&self, sample_rate: u32) -> AudioClip
    {
        if self.sample_rate == sample_rate
        {
            return self.clone();
        }

        todo!();
    }
}