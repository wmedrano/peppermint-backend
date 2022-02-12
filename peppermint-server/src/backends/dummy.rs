pub fn sample_rate_and_buffer_size() -> (f64, usize) {
    (44100.0, 1024)
}

pub fn run(peppermint: peppermint_core::PeppermintCore, buffer_size: usize) {
    let mut peppermint = peppermint;
    let mut out = peppermint_core::channels::FixedChannels::<2>::new(buffer_size);
    loop {
        // Add a delay to decrease the CPU usage.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let io = peppermint_core::IO {
            audio_out: &mut out,
            midi: std::iter::empty(),
        };
        peppermint.process(io, buffer_size);
    }
}
