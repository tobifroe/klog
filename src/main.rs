pub mod k8s;
pub mod traits;
pub mod util;

use clap::{ArgAction, Parser};
use kube::Client;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

use crate::k8s::{K8sClient, RealK8sClient, ResourceInfo, ResourceType};

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
#[derive(Clone)]
struct PodManager {
    active_pods: Arc<RwLock<HashSet<String>>>,
    resources: Arc<Vec<ResourceInfo>>,
    namespace: String,
    client: Arc<dyn K8sClient>,
    follow: bool,
    filter: String,
    shutdown: CancellationToken,
}

impl PodManager {
    fn new(
        resources: Vec<ResourceInfo>,
        namespace: String,
        client: Arc<dyn K8sClient>,
        follow: bool,
        filter: String,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            active_pods: Arc::new(RwLock::new(HashSet::new())),
            resources: Arc::new(resources),
            namespace,
            client,
            follow,
            filter,
            shutdown,
        }
    }

    async fn start_pod_logs(&self, pod_name: String) -> anyhow::Result<()> {
        let client = self.client.clone();
        let namespace = self.namespace.clone();
        let follow = self.follow;
        let filter = self.filter.clone();

        task::spawn(async move {
            if let Err(e) = client
                .stream_pod_logs(&pod_name, &namespace, follow, &filter)
                .await
            {
                eprintln!("Error streaming logs for pod {}: {}", pod_name, e);
            }
        });

        Ok(())
    }

    async fn discover_and_start_new_pods(&self) -> anyhow::Result<()> {
        let mut new_pods = Vec::new();

        for resource_info in self.resources.iter() {
            let pods = self.client.pods_for_resource(resource_info).await?;
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
        let shutdown = self.shutdown.clone();

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(e) = self.discover_and_start_new_pods().await {
                        eprintln!("Error during periodic pod discovery: {}", e);
                    }
                }
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.deployments.is_empty()
        && args.statefulsets.is_empty()
        && args.daemonsets.is_empty()
        && args.jobs.is_empty()
        && args.cronjobs.is_empty()
        && args.pods.is_empty()
    {
        anyhow::bail!("Specify at least one pod or Kubernetes resource to stream logs from");
    }

    let client = Client::try_default().await?;
    let k8s_client: Arc<dyn K8sClient> = Arc::new(RealK8sClient::new(client.clone()));
    let shutdown = CancellationToken::new();

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
        args.namespace.clone(),
        k8s_client.clone(),
        args.follow,
        args.filter.clone(),
        shutdown.clone(),
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
        let refresh_interval = args.refresh_interval;
        task::spawn(async move {
            if let Err(e) = pod_manager_clone.run_periodic_refresh(refresh_interval).await {
                eprintln!("Periodic refresh task failed: {}", e);
            }
        });
    }

    signal::ctrl_c().await?;
    shutdown.cancel();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tokio::time::timeout;

    use crate::k8s::K8sClient;

    struct MockK8s {
        pods_by_resource: Mutex<HashMap<String, Vec<String>>>,
        streamed: Mutex<Vec<(String, String, bool, String)>>,
    }

    impl MockK8s {
        fn new(pods_by_resource: HashMap<String, Vec<String>>) -> Self {
            Self {
                pods_by_resource: Mutex::new(pods_by_resource),
                streamed: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl K8sClient for MockK8s {
        async fn pods_for_resource(&self, resource: &ResourceInfo) -> Result<Vec<String>, anyhow::Error> {
            let key = match &resource.resource_type {
                ResourceType::Deployment(name)
                | ResourceType::StatefulSet(name)
                | ResourceType::DaemonSet(name)
                | ResourceType::Job(name)
                | ResourceType::CronJob(name) => name.clone(),
            };
            let map = self.pods_by_resource.lock().unwrap();
            Ok(map.get(&key).cloned().unwrap_or_default())
        }

        async fn stream_pod_logs(
            &self,
            pod_name: &str,
            ns_name: &str,
            follow: bool,
            filter: &str,
        ) -> Result<(), anyhow::Error> {
            let mut streamed = self.streamed.lock().unwrap();
            streamed.push((
                pod_name.to_string(),
                ns_name.to_string(),
                follow,
                filter.to_string(),
            ));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_discover_starts_new_pods() -> Result<(), anyhow::Error> {
        let mock = Arc::new(MockK8s::new(HashMap::from([(
            "deploy1".to_string(),
            vec!["pod-a".to_string()],
        )])));
        let resources = vec![ResourceInfo {
            resource_type: ResourceType::Deployment("deploy1".to_string()),
            namespace: "test-ns".to_string(),
        }];

        let manager = PodManager::new(
            resources,
            "test-ns".to_string(),
            mock.clone(),
            true,
            "".to_string(),
            CancellationToken::new(),
        );

        manager.discover_and_start_new_pods().await?;

        let active = manager.active_pods.read().await;
        assert!(active.contains("pod-a"));

        tokio::time::sleep(Duration::from_millis(5)).await;
        let streamed = mock.streamed.lock().unwrap();
        assert_eq!(streamed.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_discover_skips_existing_pods() -> Result<(), anyhow::Error> {
        let mock = Arc::new(MockK8s::new(HashMap::from([(
            "deploy1".to_string(),
            vec!["pod-a".to_string()],
        )])));
        let resources = vec![ResourceInfo {
            resource_type: ResourceType::Deployment("deploy1".to_string()),
            namespace: "test-ns".to_string(),
        }];

        let manager = PodManager::new(
            resources,
            "test-ns".to_string(),
            mock.clone(),
            true,
            "".to_string(),
            CancellationToken::new(),
        );
        {
            let mut active = manager.active_pods.write().await;
            active.insert("pod-a".to_string());
        }

        manager.discover_and_start_new_pods().await?;
        let streamed = mock.streamed.lock().unwrap();
        assert!(streamed.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_start_pod_logs_uses_namespace() -> Result<(), anyhow::Error> {
        let mock = Arc::new(MockK8s::new(HashMap::new()));
        let manager = PodManager::new(
            vec![],
            "custom-ns".to_string(),
            mock.clone(),
            true,
            "filter".to_string(),
            CancellationToken::new(),
        );

        manager.start_pod_logs("pod-123".to_string()).await?;
        tokio::time::sleep(Duration::from_millis(5)).await;

        let streamed = mock.streamed.lock().unwrap();
        assert_eq!(streamed.len(), 1);
        assert_eq!(streamed[0].1, "custom-ns");
        assert_eq!(streamed[0].3, "filter");
        Ok(())
    }

    #[tokio::test]
    async fn test_refresh_interval_zero_disables_refresh() -> Result<(), anyhow::Error> {
        let mock = Arc::new(MockK8s::new(HashMap::new()));
        let resources = vec![];
        let manager = PodManager::new(
            resources,
            "test-ns".to_string(),
            mock,
            true,
            "".to_string(),
            CancellationToken::new(),
        );

        let result = manager.run_periodic_refresh(0).await;
        assert!(result.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_refresh_respects_shutdown() -> Result<(), anyhow::Error> {
        let mock = Arc::new(MockK8s::new(HashMap::new()));
        let shutdown = CancellationToken::new();
        let manager = PodManager::new(
            vec![],
            "test-ns".to_string(),
            mock,
            true,
            "".to_string(),
            shutdown.clone(),
        );

        let handle = task::spawn(async move { manager.run_periodic_refresh(1).await });
        shutdown.cancel();
        let result = timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());
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
        let mock = Arc::new(MockK8s::new(HashMap::new()));
        let resources = vec![ResourceInfo {
            resource_type: ResourceType::Deployment("test-deploy".to_string()),
            namespace: "test-namespace".to_string(),
        }];

        let pod_manager = PodManager::new(
            resources,
            "test-namespace".to_string(),
            mock.clone(),
            true,
            "".to_string(),
            CancellationToken::new(),
        );

        let result = pod_manager.start_pod_logs("test-pod".to_string()).await;
        assert!(result.is_ok());

        tokio::time::sleep(Duration::from_millis(10)).await;

        let streamed = mock.streamed.lock().unwrap();
        assert_eq!(streamed.len(), 1);

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
