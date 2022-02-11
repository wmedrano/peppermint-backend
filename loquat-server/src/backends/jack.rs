use log::info;

pub fn sample_rate_and_buffer_size() -> Result<(f64, usize), jack::Error> {
    let (client, _) = jack::Client::new("loquat_probe", jack::ClientOptions::NO_START_SERVER)?;
    Ok((client.sample_rate() as f64, client.buffer_size() as usize))
}

pub fn run(loquat: loquat_core::LoquatCore) -> Result<(), jack::Error> {
    let (client, status) = jack::Client::new("loquat", jack::ClientOptions::NO_START_SERVER)?;
    info!("Started client {} with status {:?}.", client.name(), status);
    let processor = Processor {
        midi_in: client.register_port("midi_in", jack::MidiIn::default())?,
        outputs: [
            client.register_port("out_left", jack::AudioOut::default())?,
            client.register_port("out_right", jack::AudioOut::default())?,
        ],
        out_buffer: loquat_core::channels::FixedChannels::new(client.buffer_size() as usize),
        inner: loquat,
    };
    let client = client.activate_async((), processor)?;
    client
        .as_client()
        .connect_ports_by_name("loquat:out_left", "system:playback_1")
        .ok();
    client
        .as_client()
        .connect_ports_by_name("loquat:out_right", "system:playback_2")
        .ok();
    client
        .as_client()
        .connect_ports_by_name(
            "a2j:Arturia MicroLab [32] (capture): Arturia MicroLab ",
            "loquat:midi_in",
        )
        .ok();
    std::thread::park();
    client.deactivate()?;
    Ok(())
}

struct Processor {
    midi_in: jack::Port<jack::MidiIn>,
    outputs: [jack::Port<jack::AudioOut>; 2],
    out_buffer: loquat_core::channels::FixedChannels<2>,
    inner: loquat_core::LoquatCore,
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
