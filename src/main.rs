pub mod k8s;

use clap::{ArgAction, Parser};
use kube::Client;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Namespace to use
    #[arg(short, long)]
    namespace: String,

    /// Pod to log
    #[arg(short, long)]
    pod: String,

    /// Follow log?
    #[arg(short, long, action=ArgAction::SetTrue)]
    follow: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let client = Client::try_default().await?;

    k8s::stream_single_pod_logs(client, &args.pod, &args.namespace, &args.follow).await?;

    return Ok(());
}
