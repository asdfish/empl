use {
    crate::tasks::{ChannelError, TaskError, audio_error::AudioError},
    std::{ffi::OsStr, fs::File, path::Path, sync::Arc},
    symphonia::{
        core::{
            audio::SampleBuffer,
            codecs::{CodecRegistry, DecoderOptions},
            errors::Error as SymphoniaError,
            formats::FormatOptions,
            io::{MediaSourceStream, MediaSourceStreamOptions},
            meta::MetadataOptions,
            probe::{Hint, ProbeResult},
        },
        default::{get_codecs, get_probe},
    },
    tinyaudio::{OutputDevice, OutputDeviceParameters, run_output_device},
    tokio::sync::mpsc,
};

pub struct AudioTask {
    pub audio_action_rx: Option<mpsc::UnboundedReceiver<AudioAction>>,
    pub audio_error_tx: mpsc::UnboundedSender<AudioError>,
    device: Option<OutputDevice>,
}
impl AudioTask {
    pub fn new(
        audio_action_rx: mpsc::UnboundedReceiver<AudioAction>,
        audio_error_tx: mpsc::UnboundedSender<AudioError>,
    ) -> Self {
        Self {
            audio_action_rx: Some(audio_action_rx),
            audio_error_tx,
            device: None,
        }
    }
    pub fn reset(
        &mut self,
        audio_action_rx: mpsc::UnboundedReceiver<AudioAction>,
        audio_error_tx: mpsc::UnboundedSender<AudioError>,
    ) {
        self.audio_action_rx = Some(audio_action_rx);
        self.audio_error_tx = audio_error_tx;
        if let Some(device) = &mut self.device {
            device.close();
        }
        self.device = None;
    }

    pub fn spawn<'a>(&mut self) -> Result<(), TaskError<'a>> {
        let mut audio_action_rx = self
            .audio_action_rx
            .take()
            .ok_or(ChannelError::Audio(None))?;
        let audio_error_tx = self.audio_error_tx.clone();
        let config = OutputDeviceParameters {
            sample_rate: 44_100,
            channels_count: 2,
            channel_sample_count: 4_410,
        };

        let mut streamer = None;
        let mut sample_buffer = None;
        self.device = run_output_device(config, move |output| {
            if let Ok(action) = audio_action_rx.try_recv() {
                match action {
                    AudioAction::Play(path) => {
                        let file = match File::open(&path) {
                            Ok(f) => f,
                            Err(err) => {
                                let _ = audio_error_tx
                                    .send(AudioError::Decoder(SymphoniaError::IoError(err)));
                                return;
                            }
                        };

                        let source = MediaSourceStream::new(
                            Box::new(file),
                            MediaSourceStreamOptions::default(),
                        );

                        let mut hint = Hint::new();
                        if let Some(extension) = path.extension().and_then(OsStr::to_str) {
                            hint.with_extension(extension);
                        }

                        let ProbeResult {
                            format, ..
                        } = match get_probe().format(
                            &hint,
                            source,
                            &FormatOptions {
                                enable_gapless: true,
                                ..Default::default()
                            },
                            &MetadataOptions::default(),
                        ) {
                            Ok(result) => result,
                            Err(err) => {
                                let _ = audio_error_tx.send(AudioError::Decoder(err));
                                return;
                            }
                        };

                        let Some(track) = format.default_track() else {
                            let _ = audio_error_tx.send(AudioError::NoTracks);
                            return;
                        };

                        let decoder = match get_codecs().make(&track.codec_params, &DecoderOptions::default()) {
                            Ok(d) => d,
                            Err(err) => {
                                let _ = audio_error_tx.send(AudioError::Decoder(err));
                                return;
                            },
                        };

                        streamer = Some((format, decoder));
                    }
                }
            }

            let Some((format, decoder)) = streamer.as_mut() else {
                return
            };
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::ResetRequired) => {
                    let Some(track) = format.default_track() else {
                        let _ = audio_error_tx.send(AudioError::NoTracks);
                        streamer = None;
                        sample_buffer = None;
                        return;
                    };

                    let new_decoder = match CodecRegistry::new()
                        .make(&track.codec_params, &DecoderOptions::default()) {
                            Ok(d) => d,
                            Err(err) => {
                                let _ = audio_error_tx.send(AudioError::Decoder(err));
                                streamer = None;
                                sample_buffer = None;
                                return;
                            },
                        };
                    *decoder = new_decoder;

                    match format.next_packet() {
                        Ok(p) => p,
                        Err(err) => {
                            let _ = audio_error_tx.send(AudioError::Decoder(err));
                            streamer = None;
                            sample_buffer = None;
                            return;
                        }
                    }
                },
                Err(err) => {
                    let _ = audio_error_tx.send(AudioError::Decoder(err));
                    streamer = None;
                    sample_buffer = None;
                    return;
                },
            };

            let decoded_packet = match decoder.decode(&packet) {
                Ok(dp) => dp,
                Err(SymphoniaError::DecodeError(_) | SymphoniaError::IoError(_)) => return,
                Err(SymphoniaError::ResetRequired) => {
                    decoder.reset();
                    return;
                },
                Err(err) => {
                    let _ = audio_error_tx.send(AudioError::Decoder(err));
                    streamer = None;
                    sample_buffer = None;
                    return;
                },
            };

            let sample_buffer = sample_buffer
                .get_or_insert_with(|| {
                    SampleBuffer::<f32>::new(decoded_packet.capacity() as u64, *decoded_packet.spec())
                });
            sample_buffer.copy_interleaved_ref(decoded_packet);

            sample_buffer
                .samples()
                .iter()
                .zip(output)
                .for_each(|(sample, output)| *output = *sample);
        })
        .map(Some)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum AudioAction {
    Play(Arc<Path>),
}
