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
    use kube::Client;

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

    #[test]
    fn test_resource_type_creation() {
        let deployment = ResourceType::Deployment("test-deploy".to_string());
        let statefulset = ResourceType::StatefulSet("test-ss".to_string());
        let daemonset = ResourceType::DaemonSet("test-ds".to_string());
        let job = ResourceType::Job("test-job".to_string());
        let cronjob = ResourceType::CronJob("test-cj".to_string());

        match deployment {
            ResourceType::Deployment(name) => assert_eq!(name, "test-deploy"),
            _ => panic!("Expected Deployment"),
        }

        match statefulset {
            ResourceType::StatefulSet(name) => assert_eq!(name, "test-ss"),
            _ => panic!("Expected StatefulSet"),
        }

        match daemonset {
            ResourceType::DaemonSet(name) => assert_eq!(name, "test-ds"),
            _ => panic!("Expected DaemonSet"),
        }

        match job {
            ResourceType::Job(name) => assert_eq!(name, "test-job"),
            _ => panic!("Expected Job"),
        }

        match cronjob {
            ResourceType::CronJob(name) => assert_eq!(name, "test-cj"),
            _ => panic!("Expected CronJob"),
        }
    }

    #[test]
    fn test_resource_info_creation() {
        let resource_info = ResourceInfo {
            resource_type: ResourceType::Deployment("test-deploy".to_string()),
            namespace: "test-namespace".to_string(),
        };

        assert_eq!(resource_info.namespace, "test-namespace");
        match resource_info.resource_type {
            ResourceType::Deployment(name) => assert_eq!(name, "test-deploy"),
            _ => panic!("Expected Deployment"),
        }
    }

    #[tokio::test]
    async fn test_pod_manager_creation() -> Result<(), anyhow::Error> {
        let client = Client::try_default().await?;
        let resources = vec![ResourceInfo {
            resource_type: ResourceType::Deployment("test-deploy".to_string()),
            namespace: "test-namespace".to_string(),
        }];

        let pod_manager = PodManager::new(
            resources,
            client,
            true,
            "test-filter".to_string(),
        );

        assert!(pod_manager.follow);
        assert_eq!(pod_manager.filter, "test-filter");
        assert_eq!(pod_manager.resources.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_pod_manager_clone() -> Result<(), anyhow::Error> {
        let client = Client::try_default().await?;
        let resources = vec![ResourceInfo {
            resource_type: ResourceType::Deployment("test-deploy".to_string()),
            namespace: "test-namespace".to_string(),
        }];

        let pod_manager = PodManager::new(
            resources,
            client,
            true,
            "test-filter".to_string(),
        );

        let cloned_manager = pod_manager.clone();
        assert_eq!(pod_manager.follow, cloned_manager.follow);
        assert_eq!(pod_manager.filter, cloned_manager.filter);

        Ok(())
    }

    #[tokio::test]
    async fn test_refresh_interval_zero_disables_refresh() -> Result<(), anyhow::Error> {
        let client = Client::try_default().await?;
        let resources = vec![ResourceInfo {
            resource_type: ResourceType::Deployment("test-deploy".to_string()),
            namespace: "test-namespace".to_string(),
        }];

        let pod_manager = PodManager::new(
            resources,
            client,
            true,
            "".to_string(),
        );

        // This should return immediately without error
        let result = pod_manager.run_periodic_refresh(0).await;
        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_active_pods_tracking() -> Result<(), anyhow::Error> {
        let client = Client::try_default().await?;
        let resources = vec![ResourceInfo {
            resource_type: ResourceType::Deployment("test-deploy".to_string()),
            namespace: "test-namespace".to_string(),
        }];

        let pod_manager = PodManager::new(
            resources,
            client,
            true,
            "".to_string(),
        );

        // Initially no active pods
        let active_pods = pod_manager.active_pods.read().await;
        assert!(active_pods.is_empty());
        drop(active_pods);

        // Add a pod manually
        let mut active_pods = pod_manager.active_pods.write().await;
        active_pods.insert("test-pod-1".to_string());
        active_pods.insert("test-pod-2".to_string());
        drop(active_pods);

        // Check pods are tracked
        let active_pods = pod_manager.active_pods.read().await;
        assert!(active_pods.contains("test-pod-1"));
        assert!(active_pods.contains("test-pod-2"));
        assert_eq!(active_pods.len(), 2);

        Ok(())
    }

    #[test]
    fn test_args_parsing_with_refresh_interval() {
        let args = Args::try_parse_from(&[
            "klog",
            "--namespace", "test-ns",
            "--refresh-interval", "60"
        ]).unwrap();

        assert_eq!(args.namespace, "test-ns");
        assert_eq!(args.refresh_interval, 60);
    }

    #[test]
    fn test_args_parsing_with_default_refresh_interval() {
        let args = Args::try_parse_from(&[
            "klog",
            "--namespace", "test-ns"
        ]).unwrap();

        assert_eq!(args.namespace, "test-ns");
        assert_eq!(args.refresh_interval, 30); // default value
    }

    #[test]
    fn test_args_parsing_disable_refresh() {
        let args = Args::try_parse_from(&[
            "klog",
            "--namespace", "test-ns",
            "--refresh-interval", "0"
        ]).unwrap();

        assert_eq!(args.namespace, "test-ns");
        assert_eq!(args.refresh_interval, 0);
    }

    #[tokio::test]
    async fn test_start_pod_logs_spawns_task() -> Result<(), anyhow::Error> {
        let client = Client::try_default().await?;
        let resources = vec![ResourceInfo {
            resource_type: ResourceType::Deployment("test-deploy".to_string()),
            namespace: "test-namespace".to_string(),
        }];

        let pod_manager = PodManager::new(
            resources,
            client,
            true,
            "".to_string(),
        );

        let result = pod_manager.start_pod_logs("test-pod".to_string()).await;
        assert!(result.is_ok());

        tokio::time::sleep(Duration::from_millis(10)).await;

        Ok(())
    }

    #[test]
    fn test_resource_type_debug() {
        let deployment = ResourceType::Deployment("test-deploy".to_string());
        let debug_str = format!("{:?}", deployment);
        assert!(debug_str.contains("Deployment"));
        assert!(debug_str.contains("test-deploy"));
    }

    #[test]
    fn test_resource_info_clone() {
        let original = ResourceInfo {
            resource_type: ResourceType::Deployment("test-deploy".to_string()),
            namespace: "test-namespace".to_string(),
        };

        let cloned = original.clone();
        assert_eq!(original.namespace, cloned.namespace);
        match (&original.resource_type, &cloned.resource_type) {
            (ResourceType::Deployment(name1), ResourceType::Deployment(name2)) => {
                assert_eq!(name1, name2);
            }
            _ => panic!("Expected both to be Deployments"),
        }
    }
}
