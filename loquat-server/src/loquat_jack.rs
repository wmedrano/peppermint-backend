pub struct Processor {
    midi_in: jack::Port<jack::MidiIn>,
    outputs: [jack::Port<jack::AudioOut>; 2],
    out_buffer: loquat_core::channels::FixedChannels<2>,
    inner: loquat_core::LoquatCore,
}

impl Processor {
    pub fn new(
        client: &jack::Client,
        loquat: loquat_core::LoquatCore,
    ) -> Result<Processor, jack::Error> {
        Ok(Processor {
            midi_in: client.register_port("midi_in", jack::MidiIn::default())?,
            outputs: [
                client.register_port("out_left", jack::AudioOut::default())?,
                client.register_port("out_right", jack::AudioOut::default())?,
            ],
            out_buffer: loquat_core::channels::FixedChannels::new(client.buffer_size() as usize),
            inner: loquat,
        })
    }
}

impl jack::ProcessHandler for Processor {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        let io = loquat_core::IO {
            audio_out: &mut self.out_buffer,
            midi: self.midi_in.iter(ps).map(|m| loquat_core::RawMidi {
                frame: m.time as usize,
                data: m.bytes,
            }),
        };
        self.inner.process(io, ps.n_frames() as usize);
        let srcs = self.out_buffer.iter_channels();
        let dsts = self.outputs.iter_mut();
        for (src, dst) in srcs.zip(dsts) {
            dst.as_mut_slice(ps).copy_from_slice(src);
        }
        jack::Control::Continue
    }

    fn buffer_size(&mut self, _: &jack::Client, buffer_size: jack::Frames) -> jack::Control {
        self.out_buffer.set_buffer_size(buffer_size as usize);
        self.inner.set_buffer_size(buffer_size as usize);
        jack::Control::Continue
    }
}
