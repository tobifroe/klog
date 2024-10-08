use std::collections::BTreeMap;

use anyhow::Ok;
use colored::Colorize;
use futures_util::AsyncBufReadExt;
use futures_util::TryStreamExt;
use k8s_openapi::api::apps::v1::DaemonSet;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, ListParams, LogParams};
use kube::runtime::reflector::Lookup;
use kube::ResourceExt;

use itertools::Itertools;

use crate::util;

async fn get_pod_list(
    client: &kube::Client,
    ns_name: &str,
    match_labels: BTreeMap<String, String>,
) -> Result<Vec<String>, anyhow::Error> {
    let labels: String = match_labels
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .join(",");

    let pod_api: Api<Pod> = Api::namespaced(client.clone(), ns_name);
    let list_params = ListParams::default().labels(&labels);
    let pod_list = pod_api.list(&list_params).await?;

    let mut pod_name_list: std::vec::Vec<std::string::String> = vec![];
    for pod in pod_list.iter() {
        pod_name_list.push(pod.name().unwrap().to_string());
    }
    Ok(pod_name_list)
}

pub async fn get_pod_list_for_deployment(
    client: &kube::Client,
    deployment_name: &str,
    ns_name: &str,
) -> Result<Vec<String>, anyhow::Error> {
    let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), ns_name);
    let deployment = deployment_api.get(deployment_name).await?;

    let spec = deployment.spec.unwrap();
    let match_labels = spec.selector.match_labels.unwrap();

    let pod_name_list = get_pod_list(client, ns_name, match_labels).await?;
    Ok(pod_name_list)
}

pub async fn get_pod_list_for_statefulset(
    client: &kube::Client,
    statefulset_name: &str,
    ns_name: &str,
) -> Result<Vec<String>, anyhow::Error> {
    let statefulset_api: Api<StatefulSet> = Api::namespaced(client.clone(), ns_name);
    let statefulset = statefulset_api.get(statefulset_name).await?;

    let spec = statefulset.spec.unwrap();
    let match_labels = spec.selector.match_labels.unwrap();

    let pod_name_list = get_pod_list(client, ns_name, match_labels).await?;
    Ok(pod_name_list)
}

pub async fn get_pod_list_for_daemonset(
    client: &kube::Client,
    daemonset_name: &str,
    ns_name: &str,
) -> Result<Vec<String>, anyhow::Error> {
    let ds_api: Api<DaemonSet> = Api::namespaced(client.clone(), ns_name);
    let ds = ds_api.get(daemonset_name).await?;

    let spec = ds.spec.unwrap();
    let match_labels = spec.selector.match_labels.unwrap();

    let pod_name_list = get_pod_list(client, ns_name, match_labels).await?;
    Ok(pod_name_list)
}

pub async fn stream_single_pod_logs(
    client: &kube::Client,
    pod_name: &str,
    ns_name: &str,
    follow: &bool,
) -> Result<(), anyhow::Error> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), ns_name);
    let pod = pods.get(pod_name).await?;

    let spec = &pod.spec.clone().unwrap();
    let container = &spec.containers.first();
    let name = &container.unwrap().name;
    let mut logs = pods
        .log_stream(
            pod_name,
            &LogParams {
                follow: *follow,
                pretty: true,
                container: Some(name.clone()),
                ..LogParams::default()
            },
        )
        .await?
        .lines();

    let color = util::get_rnd_color();

    while let Some(line) = logs.try_next().await? {
        let pretty_pod_name = &pod.name_any().truecolor(color.r, color.g, color.b);
        println!("{} {}", pretty_pod_name, line);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube::Client;

    #[tokio::test]
    async fn test_get_pod_list() {
        let expected_pod_list_item = "web-0";
        let client_result = Client::try_default().await;
        let client = client_result.unwrap();

        let ns_name = "statefulset";
        let statefulset_name = "web";
        let statefulset_api: Api<StatefulSet> = Api::namespaced(client.clone(), ns_name);
        let statefulset = statefulset_api.get(statefulset_name).await;

        let spec = statefulset.unwrap().spec.unwrap();
        let match_labels = spec.selector.match_labels.unwrap();
        let pod_list_result = get_pod_list(&client, "statefulset", match_labels).await;
        let pod_list = pod_list_result.unwrap();
        assert_eq!(pod_list.first().unwrap(), expected_pod_list_item);
    }
}
