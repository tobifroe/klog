pub mod k8s;
pub mod traits;
pub mod util;

use clap::{ArgAction, Parser};
use k8s_openapi::api::{
    apps::v1::{DaemonSet, Deployment, StatefulSet},
    batch::v1::{CronJob, Job},
};
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

    /// Statefulsets to log
    #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
    statefulsets: Vec<String>,

    /// Daemonsets to log
    #[arg(long, value_delimiter = ' ', num_args = 1..)]
    daemonsets: Vec<String>,

    /// Jobs to log
    #[arg(long, value_delimiter = ' ', num_args = 1..)]
    jobs: Vec<String>,

    /// CronJobs to log
    #[arg(long, value_delimiter = ' ', num_args = 1..)]
    cronjobs: Vec<String>,

    /// Pods to log
    #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
    pods: Vec<String>,

    /// Follow log?
    #[arg(short, long, action = ArgAction::SetTrue)]
    follow: bool,

    /// Filter
    #[arg(long, default_value = "")]
    filter: String,
}
enum ResourceType<'a> {
    Deployment(&'a str),
    StatefulSet(&'a str),
    DaemonSet(&'a str),
    Job(&'a str),
    CronJob(&'a str),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let client = Client::try_default().await?;

    let mut pod_list = args.pods;

    let mut resources = Vec::new();

    resources.extend(
        args.deployments
            .iter()
            .map(|deploy| ResourceType::Deployment(deploy)),
    );
    resources.extend(
        args.statefulsets
            .iter()
            .map(|statefulset| ResourceType::StatefulSet(statefulset)),
    );
    resources.extend(args.daemonsets.iter().map(|ds| ResourceType::DaemonSet(ds)));
    resources.extend(args.jobs.iter().map(|job| ResourceType::Job(job)));

    for resource in resources {
        match resource {
            ResourceType::Deployment(deploy) => {
                pod_list.append(
                    &mut k8s::get_pod_list_for_resource::<Deployment>(
                        &client,
                        deploy,
                        &args.namespace,
                    )
                    .await?,
                );
            }
            ResourceType::StatefulSet(statefulset) => {
                pod_list.append(
                    &mut k8s::get_pod_list_for_resource::<StatefulSet>(
                        &client,
                        statefulset,
                        &args.namespace,
                    )
                    .await?,
                );
            }
            ResourceType::DaemonSet(ds) => {
                pod_list.append(
                    &mut k8s::get_pod_list_for_resource::<DaemonSet>(&client, ds, &args.namespace)
                        .await?,
                );
            }
            ResourceType::Job(job) => {
                pod_list.append(
                    &mut k8s::get_pod_list_for_resource::<Job>(&client, job, &args.namespace)
                        .await?,
                );
            }
            ResourceType::CronJob(cronjob) => pod_list.append(
                &mut k8s::get_pod_list_for_resource::<CronJob>(&client, cronjob, &args.namespace)
                    .await?,
            ),
        }
    }

    let namespace = args.namespace;
    let follow = args.follow;
    let filter = args.filter;

    let mut handles = Vec::new();

    for pod in pod_list {
        let client = client.clone();
        let namespace = namespace.clone();
        let filter = filter.clone();

        let handle = task::spawn(async move {
            k8s::stream_single_pod_logs(&client, &pod, &namespace, &follow, &filter).await?;
            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_processing() -> Result<(), anyhow::Error> {
        let args = Args {
            deployments: vec!["deploy1".into()],
            statefulsets: vec!["statefulset1".into()],
            daemonsets: vec!["daemonset1".into()],
            jobs: vec!["job1".into()],
            cronjobs: vec!["job2".into()],
            pods: vec!["pod1".into()],
            namespace: "test-namespace".into(),
            follow: true,
            filter: "".into(),
        };

        let resources: Vec<_> = args
            .deployments
            .iter()
            .map(|deploy| ResourceType::Deployment(deploy))
            .chain(
                args.statefulsets
                    .iter()
                    .map(|statefulset| ResourceType::StatefulSet(statefulset)),
            )
            .chain(args.daemonsets.iter().map(|ds| ResourceType::DaemonSet(ds)))
            .chain(args.jobs.iter().map(|job| ResourceType::Job(job)))
            .chain(
                args.cronjobs
                    .iter()
                    .map(|cronjob| ResourceType::CronJob(cronjob)),
            )
            .collect();

        assert_eq!(resources.len(), 5);
        match resources[0] {
            ResourceType::Deployment(deploy) => assert_eq!(deploy, "deploy1"),
            _ => panic!("Expected Deployment"),
        }

        Ok(())
    }
}
