use log::info;

pub mod loquat_jack;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let addr = "[::1]:50218".parse()?;
    let loquat = LoquatServiceImpl::new();

    info!("Runing loquat server on {}", addr);
    let server = tonic::transport::Server::builder()
        .add_service(loquat_proto::loquat_server::LoquatServer::new(loquat))
        .serve(addr);

    let (client, status) =
        jack::Client::new("loquat", jack::ClientOptions::NO_START_SERVER).unwrap();
    info!("Started client {} with status {:?}.", client.name(), status);
    let processor = loquat_jack::Processor::new(&client).unwrap();
    let client = client.activate_async((), processor).unwrap();

    server.await?;
    client.deactivate().unwrap();
    Ok(())
}

struct LoquatServiceImpl {}

impl LoquatServiceImpl {
    pub fn new() -> Self {
        LoquatServiceImpl {}
    }
}

#[tonic::async_trait]
impl loquat_proto::loquat_server::Loquat for LoquatServiceImpl {
    async fn get_plugins(
        &self,
        _: tonic::Request<loquat_proto::GetPluginsRequest>,
    ) -> Result<tonic::Response<loquat_proto::GetPluginsResponse>, tonic::Status> {
        let plugins = vec![];
        Ok(tonic::Response::new(loquat_proto::GetPluginsResponse {
            plugins,
        }))
    }
}
