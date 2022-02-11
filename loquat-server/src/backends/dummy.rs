pub fn sample_rate_and_buffer_size() -> (f64, usize) {
    (44100.0, 1024)
}

pub fn run(loquat: loquat_core::LoquatCore, buffer_size: usize) {
    let mut loquat = loquat;
    let mut out = loquat_core::channels::FixedChannels::<2>::new(buffer_size);
    loop {
        // Add a delay to decrease the CPU usage.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let io = loquat_core::IO {
            audio_out: &mut out,
            midi: std::iter::empty(),
        };
        loquat.process(io, buffer_size);
    }
}
