pub mod k8s;
pub mod util;

use clap::{ArgAction, Parser};
use kube::Client;
use tokio::task;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Namespace to use
    #[arg(short, long)]
    namespace: String,

    /// Deployment to log
    #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
    deployments: Vec<String>,

    /// Pods to log
    #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
    pods: Vec<String>,

    /// Follow log?
    #[arg(short, long, action = ArgAction::SetTrue)]
    follow: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let client = Client::try_default().await?;

    let mut pod_list = args.pods;

    if args.deployments.len() > 0 {
        for deploy in args.deployments.iter() {
            pod_list.append(
                &mut k8s::get_pod_list_for_deployment(&client, &deploy, &args.namespace).await?,
            );
        }
    }

    let namespace = args.namespace;
    let follow = args.follow;

    let mut handles = Vec::new();

    for pod in pod_list {
        let client = client.clone();
        let namespace = namespace.clone();

        let handle = task::spawn(async move {
            k8s::stream_single_pod_logs(&client, &pod, &namespace, &follow).await?;
            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}
