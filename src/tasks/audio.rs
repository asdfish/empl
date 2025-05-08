use {
    crate::tasks::{ChannelError, TaskError},
    std::{ffi::OsStr, fs::File, path::Path, sync::Arc},
    symphonia::{
        core::{
            formats::FormatOptions,
            io::{MediaSourceStream, MediaSourceStreamOptions},
            meta::MetadataOptions,
            probe::{
                Hint,
                ProbeResult,
            },
        },
        default::get_probe,
    },
    tinyaudio::{OutputDevice, OutputDeviceParameters, run_output_device},
    tokio::sync::mpsc,
};

pub struct AudioTask {
    pub audio_action_rx: Option<mpsc::UnboundedReceiver<AudioAction>>,
    device: Option<OutputDevice>,
}
impl AudioTask {
    pub fn new(audio_action_rx: mpsc::UnboundedReceiver<AudioAction>) -> Self {
        Self {
            audio_action_rx: Some(audio_action_rx),
            device: None,
        }
    }
    pub fn reset(&mut self, audio_action_rx: mpsc::UnboundedReceiver<AudioAction>) {
        self.audio_action_rx = Some(audio_action_rx);
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
        let config = OutputDeviceParameters {
            sample_rate: 44_100,
            channels_count: 2,
            channel_sample_count: 4_410,
        };

        self.device = run_output_device(
            config,
            move |_output| {
                if let Ok(action) = audio_action_rx.try_recv() {
                    match action {
                        AudioAction::Play(path) => {
                            let file = match File::open(&path) {
                                Ok(f) => f,
                                Err(_err) => {
                                    todo!("send error")
                                },
                            };

                            let source = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

                            let mut hint = Hint::new();
                            if let Some(extension) = path.extension().and_then(OsStr::to_str) {
                                hint.with_extension(extension);
                            }

                            let ProbeResult { format: _format, .. } = match get_probe().format(&hint, source, &FormatOptions {
                                enable_gapless: true,
                                ..Default::default()
                            }, &MetadataOptions::default()) {
                                Ok(result) => result,
                                Err(_err) => {
                                    todo!("send error")
                                }
                            };
                        }
                    }
                }
            },
        )
        .map(Some)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum AudioAction {
    Play(Arc<Path>),
}
