use log::{info, warn};
use structopt::StructOpt;

pub mod backends;
pub mod grpc_service;
pub mod manager;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(long, default_value = "50218")]
    port: u16,

    #[structopt(long, default_value = "jack")]
    backend: Backend,

    #[structopt(long, default_value = "4096")]
    command_queue_size: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let (command_tx, command_rx) =
        ringbuf::RingBuffer::<peppermint_core::command::Command>::new(options.command_queue_size)
            .split();

    let addr = format!("127.0.0.1:{}", options.port).parse()?;
    let (sample_rate, buffer_size) = match options.backend {
        Backend::Dummy => backends::dummy::sample_rate_and_buffer_size(),
        Backend::Jack => backends::jack::sample_rate_and_buffer_size().unwrap(),
    };
    let peppermint_service =
        grpc_service::PeppermintServiceImpl::new(sample_rate, buffer_size, command_tx);
    let server = tonic::transport::Server::builder()
        .add_service(peppermint_proto::peppermint_server::PeppermintServer::new(
            peppermint_service,
        ))
        .serve(addr);

    info!("Running audio loop for backend {:?}.", options.backend);
    let _audio_thread = std::thread::spawn(move || {
        let core = peppermint_core::PeppermintCore::new(command_rx);
        match options.backend {
            Backend::Dummy => backends::dummy::run(core, buffer_size),
            Backend::Jack => backends::jack::run(core).unwrap(),
        }
    });

    info!("peppermint is ready at {}.", addr);
    server.await?;
    warn!("Terminating peppermint.");
    Ok(())
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
