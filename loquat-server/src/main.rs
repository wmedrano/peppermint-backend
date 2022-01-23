use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let addr = "[::1]:50218".parse()?;
    let loquat = Loquat::new();

    info!("Runing loquat server on {}", addr);
    tonic::transport::Server::builder()
        .add_service(loquat_proto::loquat_server::LoquatServer::new(loquat))
        .serve(addr)
        .await?;

    Ok(())
}

struct Loquat {}

impl Loquat {
    pub fn new() -> Self {
        Loquat {}
    }
}

#[tonic::async_trait]
impl loquat_proto::loquat_server::Loquat for Loquat {
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
