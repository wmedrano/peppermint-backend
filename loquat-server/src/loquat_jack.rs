pub struct Processor {
    midi_in: jack::Port<jack::MidiIn>,
    outputs: [jack::Port<jack::AudioOut>; 2],
    inner: loquat_core::LoquatCore,
}

impl Processor {
    pub fn new(client: &jack::Client) -> Result<Processor, jack::Error> {
        Ok(Processor {
            midi_in: client.register_port("midi_in", jack::MidiIn::default())?,
            outputs: [
                client.register_port("out_left", jack::AudioOut::default())?,
                client.register_port("out_right", jack::AudioOut::default())?,
            ],
            inner: loquat_core::LoquatCore::new(),
        })
    }
}

impl jack::ProcessHandler for Processor {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        let [out_left, out_right] = &mut self.outputs;
        let io = loquat_core::IO {
            out_left: out_left.as_mut_slice(ps),
            out_right: out_right.as_mut_slice(ps),
            midi: self.midi_in.iter(ps).map(|m| loquat_core::RawMidi {
                frame: m.time as usize,
                data: m.bytes,
            }),
        };
        self.inner.process(io);
        jack::Control::Continue
    }

    fn buffer_size(&mut self, _: &jack::Client, _: jack::Frames) -> jack::Control {
        jack::Control::Continue
    }
}
