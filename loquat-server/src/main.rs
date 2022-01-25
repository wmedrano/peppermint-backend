use log::{info, warn};

pub mod loquat_jack;
pub mod service_impl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let command_queue_size = 4096;
    let (command_tx, command_rx) =
        ringbuf::RingBuffer::<loquat_core::command::Command>::new(command_queue_size).split();

    let (client, status) =
        jack::Client::new("loquat", jack::ClientOptions::NO_START_SERVER).unwrap();
    info!("Started client {} with status {:?}.", client.name(), status);

    let addr = "[::1]:50218".parse()?;
    info!("Runing loquat server on {}", addr);
    let loquat = service_impl::LoquatServiceImpl::new(client.buffer_size() as usize, command_tx);
    let server = tonic::transport::Server::builder()
        .add_service(loquat_proto::loquat_server::LoquatServer::new(loquat))
        .serve(addr);

    let processor = loquat_jack::Processor::new(&client, command_rx).unwrap();
    let client = client.activate_async((), processor).unwrap();

    info!("Loquat is ready.");
    server.await?;
    warn!("Terminating Loquat.");
    client.deactivate().unwrap();
    Ok(())
}
