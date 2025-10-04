pub mod k8s;
pub mod traits;
pub mod util;

use clap::{ArgAction, Parser};
use k8s_openapi::api::{
    apps::v1::{DaemonSet, Deployment, StatefulSet},
    batch::v1::{CronJob, Job},
};
use kube::Client;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::{interval, Duration};

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

    /// Refresh interval in seconds for discovering new pods (0 to disable)
    #[arg(long, default_value = "30")]
    refresh_interval: u64,
}
#[derive(Clone, Debug)]
enum ResourceType {
    Deployment(String),
    StatefulSet(String),
    DaemonSet(String),
    Job(String),
    CronJob(String),
}

#[derive(Clone)]
struct ResourceInfo {
    resource_type: ResourceType,
    namespace: String,
}

#[derive(Clone)]
struct PodManager {
    active_pods: Arc<RwLock<HashSet<String>>>,
    resources: Arc<Vec<ResourceInfo>>,
    client: Client,
    follow: bool,
    filter: String,
}

impl PodManager {
    fn new(
        resources: Vec<ResourceInfo>,
        client: Client,
        follow: bool,
        filter: String,
    ) -> Self {
        Self {
            active_pods: Arc::new(RwLock::new(HashSet::new())),
            resources: Arc::new(resources),
            client,
            follow,
            filter,
        }
    }

    async fn start_pod_logs(&self, pod_name: String) -> anyhow::Result<()> {
        let client = self.client.clone();
        let namespace = self.resources[0].namespace.clone();
        let follow = self.follow;
        let filter = self.filter.clone();

        task::spawn(async move {
            if let Err(e) = k8s::stream_single_pod_logs(&client, &pod_name, &namespace, &follow, &filter).await {
                eprintln!("Error streaming logs for pod {}: {}", pod_name, e);
            }
        });

        Ok(())
    }

    async fn discover_and_start_new_pods(&self) -> anyhow::Result<()> {
        let mut new_pods = Vec::new();

        for resource_info in self.resources.iter() {
            let pods = match &resource_info.resource_type {
                ResourceType::Deployment(name) => {
                    k8s::get_pod_list_for_resource::<Deployment>(
                        &self.client,
                        name,
                        &resource_info.namespace,
                    )
                    .await?
                }
                ResourceType::StatefulSet(name) => {
                    k8s::get_pod_list_for_resource::<StatefulSet>(
                        &self.client,
                        name,
                        &resource_info.namespace,
                    )
                    .await?
                }
                ResourceType::DaemonSet(name) => {
                    k8s::get_pod_list_for_resource::<DaemonSet>(
                        &self.client,
                        name,
                        &resource_info.namespace,
                    )
                    .await?
                }
                ResourceType::Job(name) => {
                    k8s::get_pod_list_for_resource::<Job>(
                        &self.client,
                        name,
                        &resource_info.namespace,
                    )
                    .await?
                }
                ResourceType::CronJob(name) => {
                    k8s::get_pod_list_for_resource::<CronJob>(
                        &self.client,
                        name,
                        &resource_info.namespace,
                    )
                    .await?
                }
            };

            new_pods.extend(pods);
        }

        // Check for new pods and start logging them
        let mut active_pods = self.active_pods.write().await;
        for pod in new_pods {
            if !active_pods.contains(&pod) {
                active_pods.insert(pod.clone());
                drop(active_pods); // Release the lock before starting the async task
                self.start_pod_logs(pod).await?;
                active_pods = self.active_pods.write().await; // Reacquire the lock
            }
        }

        Ok(())
    }

    async fn run_periodic_refresh(&self, interval_seconds: u64) -> anyhow::Result<()> {
        if interval_seconds == 0 {
            return Ok(());
        }

        let mut interval = interval(Duration::from_secs(interval_seconds));
        
        loop {
            interval.tick().await;
            if let Err(e) = self.discover_and_start_new_pods().await {
                eprintln!("Error during periodic pod discovery: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let client = Client::try_default().await?;

    // Build resource list
    let mut resource_infos = Vec::new();

    for deploy in &args.deployments {
        resource_infos.push(ResourceInfo {
            resource_type: ResourceType::Deployment(deploy.clone()),
            namespace: args.namespace.clone(),
        });
    }

    for statefulset in &args.statefulsets {
        resource_infos.push(ResourceInfo {
            resource_type: ResourceType::StatefulSet(statefulset.clone()),
            namespace: args.namespace.clone(),
        });
    }

    for daemonset in &args.daemonsets {
        resource_infos.push(ResourceInfo {
            resource_type: ResourceType::DaemonSet(daemonset.clone()),
            namespace: args.namespace.clone(),
        });
    }

    for job in &args.jobs {
        resource_infos.push(ResourceInfo {
            resource_type: ResourceType::Job(job.clone()),
            namespace: args.namespace.clone(),
        });
    }

    for cronjob in &args.cronjobs {
        resource_infos.push(ResourceInfo {
            resource_type: ResourceType::CronJob(cronjob.clone()),
            namespace: args.namespace.clone(),
        });
    }

    // Create pod manager
    let pod_manager = PodManager::new(
        resource_infos,
        client.clone(),
        args.follow,
        args.filter.clone(),
    );

    // Start with initial pod discovery
    pod_manager.discover_and_start_new_pods().await?;

    // Start with explicitly specified pods
    for pod in &args.pods {
        pod_manager.start_pod_logs(pod.clone()).await?;
        // Add to active pods set
        let mut active_pods = pod_manager.active_pods.write().await;
        active_pods.insert(pod.clone());
    }

    // Start periodic refresh if enabled
    if args.refresh_interval > 0 {
        let pod_manager_clone = pod_manager.clone();
        task::spawn(async move {
            if let Err(e) = pod_manager_clone.run_periodic_refresh(args.refresh_interval).await {
                eprintln!("Periodic refresh task failed: {}", e);
            }
        });
    }

    // Keep the main thread alive
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
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
            refresh_interval: 30,
        };

        let resources: Vec<_> = args
            .deployments
            .iter()
            .map(|deploy| ResourceType::Deployment(deploy.clone()))
            .chain(
                args.statefulsets
                    .iter()
                    .map(|statefulset| ResourceType::StatefulSet(statefulset.clone())),
            )
            .chain(args.daemonsets.iter().map(|ds| ResourceType::DaemonSet(ds.clone())))
            .chain(args.jobs.iter().map(|job| ResourceType::Job(job.clone())))
            .chain(
                args.cronjobs
                    .iter()
                    .map(|cronjob| ResourceType::CronJob(cronjob.clone())),
            )
            .collect();

        assert_eq!(resources.len(), 5);
        match &resources[0] {
            ResourceType::Deployment(deploy) => assert_eq!(deploy, "deploy1"),
            _ => panic!("Expected Deployment"),
        }

        Ok(())
    }
}
