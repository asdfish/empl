use {
    crate::tasks::ChannelError,
    std::{
        ffi::OsStr,
        fs::File,
        path::Path,
        sync::{Arc, mpsc as std_mpsc},
    },
    symphonia::core::{
        audio::SampleBuffer,
        codecs::{CODEC_TYPE_NULL, CodecParameters, Decoder, DecoderOptions},
        errors::Error as SymphoniaError,
        formats::{FormatOptions, FormatReader, Track},
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::MetadataOptions,
        probe::{Hint, ProbeResult},
    },
    tokio::{
        sync::mpsc as tokio_mpsc,
    },
};

fn supported_track(
    Track {
        codec_params: CodecParameters { codec, .. },
        ..
    }: &Track,
) -> bool {
    CODEC_TYPE_NULL.ne(codec)
}

fn create_decoder(stream: &Box<dyn FormatReader>) -> Option<Box<dyn Decoder>> {
    let Track { codec_params, .. } = stream
        .tracks()
        .iter()
        .find(|track| supported_track(track))?;

    symphonia::default::get_codecs()
        .make(codec_params, &DecoderOptions::default()).ok()
}

pub struct DecoderTask {
    action_rx: std_mpsc::Receiver<DecoderAction>,
    idle_tx: tokio_mpsc::UnboundedSender<()>,
    output_tx: std_mpsc::Sender<SampleBuffer<f32>>,
    decoder_and_stream: Option<(Box<dyn Decoder>, Box<dyn FormatReader>)>,
}
impl DecoderTask {
    pub const fn new(
        action_rx: std_mpsc::Receiver<DecoderAction>,
        idle_tx: tokio_mpsc::UnboundedSender<()>,
        output_tx: std_mpsc::Sender<SampleBuffer<f32>>,
    ) -> Self {
        Self {
            action_rx,
            idle_tx,
            output_tx,
            decoder_and_stream: None,
        }
    }

    pub fn run<'a>(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            match self.decoder_and_stream.as_mut() {
                Some((decoder, stream)) => match self.action_rx.try_recv() {
                    Ok(DecoderAction::End) => break Ok(()),
                    Ok(DecoderAction::Play(path)) => {
                        if self.play(&path).is_err() {
                            self.idle_tx
                                .send(())
                                .map_err(|_| ChannelError::DecoderIdle)?;
                        };
                    }
                    Err(std_mpsc::TryRecvError::Empty) => {
                        if (|| {
                            let packet = match stream.next_packet() {
                                Ok(p) => p,
                                Err(SymphoniaError::ResetRequired) => {
                                    *decoder = create_decoder(stream).ok_or(())?;
                                    return Ok(());
                                }
                                Err(_) => return Err(()),
                            };

                            let decoded_packet = match decoder.decode(&packet) {
                                Ok(dp) => dp,
                                Err(
                                    SymphoniaError::DecodeError(_) | SymphoniaError::IoError(_),
                                ) => {
                                    return Ok(());
                                }
                                Err(SymphoniaError::ResetRequired) => {
                                    decoder.reset();
                                    return Ok(());
                                }
                                Err(_) => return Err(()),
                            };

                            let mut sample_buffer =
                                SampleBuffer::new(packet.dur, *decoded_packet.spec());
                            sample_buffer.copy_interleaved_ref(decoded_packet);
                            self.output_tx.send(sample_buffer).map_err(|_| ())
                        })()
                        .is_err()
                        {
                            self.idle_tx
                                .send(())
                                .map_err(|_| ChannelError::DecoderIdle)?;
                            self.decoder_and_stream = None;
                        }
                    }
                    Err(std_mpsc::TryRecvError::Disconnected) => {
                        break Err(ChannelError::DecoderAction(None));
                    }
                },
                None => {
                    match self
                        .action_rx
                        .recv()
                        .map_err(|_| ChannelError::DecoderAction(None))?
                    {
                        DecoderAction::End => break Ok(()),
                        DecoderAction::Play(path) => {
                            if self.play(&path).is_err() {
                                self.idle_tx
                                    .send(())
                                    .map_err(|_| ChannelError::DecoderIdle)?;
                            }
                        }
                    }
                }
            }
        }
    }

    /// When it returns an [Err] you should send a message to [Self::idle_tx].
    fn play<P>(&mut self, path: &P) -> Result<(), ()>
    where
        P: AsRef<Path> + ?Sized,
    {
        let Ok(source) = File::open(path) else {
            return Err(());
        };

        let stream = MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.as_ref().extension().and_then(OsStr::to_str) {
            hint.with_extension(ext);
        }
        let Ok(ProbeResult { format, .. }) = symphonia::default::get_probe().format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) else {
            return Err(());
        };
        let decoder = create_decoder(&format).ok_or(())?;

        self.decoder_and_stream = Some((decoder, format));
        Ok(())
    }
}

#[derive(Debug)]
pub enum DecoderAction {
    End,
    Play(Arc<Path>),
}
