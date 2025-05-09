use {
    std::{
        error::Error,
        sync::mpsc,
    },
    symphonia::core::audio::SampleBuffer,
    tinyaudio::{OutputDevice, OutputDeviceParameters, run_output_device},
};

pub fn spawn(sample_rx: mpsc::Receiver<SampleBuffer<f32>>) -> Result<OutputDevice, Box<dyn Error>> {
    run_output_device(OutputDeviceParameters {
        sample_rate: 44_100,
        channels_count: 2,
        channel_sample_count: 4_410,
    }, move |output| {
        let Ok(sample) =  sample_rx.recv() else {
            return;
        };

        output
            .iter_mut()
            .zip(sample.samples())
            .for_each(|(out, sample)| *out = *sample);
    })
}
