use {
    crate::tasks::{ChannelError, TaskError},
    std::{path::Path, sync::Arc},
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
                if let Ok(_action) = audio_action_rx.try_recv() {}
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
