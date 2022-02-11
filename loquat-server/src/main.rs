use log::{info, warn};
use structopt::StructOpt;

pub mod loquat_jack;
pub mod service_impl;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(long, default_value = "50218")]
    port: u16,

    #[structopt(long, default_value = "jack")]
    backend: Backend,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let command_queue_size = 4096;
    let (command_tx, command_rx) =
        ringbuf::RingBuffer::<loquat_core::command::Command>::new(command_queue_size).split();

    let addr = format!("127.0.0.1:{}", options.port).parse()?;
    let (sample_rate, buffer_size) = sample_rate_and_buffer_size(&options);
    let loquat_service = service_impl::LoquatServiceImpl::new(sample_rate, buffer_size, command_tx);
    let server = tonic::transport::Server::builder()
        .add_service(loquat_proto::loquat_server::LoquatServer::new(
            loquat_service,
        ))
        .serve(addr);

    info!("Running audio loop for backend {:?}.", options.backend);
    let _audio_thread = std::thread::spawn(move || {
        let core = loquat_core::LoquatCore::new(command_rx);
        match options.backend {
            Backend::Dummy => run_dummy(core, buffer_size),
            Backend::Jack => run_jack(core),
        }
    });

    info!("Loquat is ready at {}.", addr);
    server.await?;
    warn!("Terminating Loquat.");
    Ok(())
}

fn sample_rate_and_buffer_size(options: &Options) -> (f64, usize) {
    match options.backend {
        Backend::Dummy => (44100.0, 1024),
        Backend::Jack => {
            let (client, _) =
                jack::Client::new("loquat_probe", jack::ClientOptions::NO_START_SERVER).unwrap();
            (client.sample_rate() as f64, client.buffer_size() as usize)
        }
    }
}

fn run_jack(loquat: loquat_core::LoquatCore) {
    let (client, status) =
        jack::Client::new("loquat", jack::ClientOptions::NO_START_SERVER).unwrap();
    info!("Started client {} with status {:?}.", client.name(), status);
    let processor = loquat_jack::Processor::new(&client, loquat).unwrap();
    let client = client.activate_async((), processor).unwrap();
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
    client.deactivate().unwrap();
}

fn run_dummy(loquat: loquat_core::LoquatCore, buffer_size: usize) {
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

#[derive(Debug)]
enum Backend {
    Dummy,
    Jack,
}

impl std::str::FromStr for Backend {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dummy" => Ok(Backend::Dummy),
            "jack" => Ok(Backend::Jack),
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid backend",
            ))),
        }
    }
}
