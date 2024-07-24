use anyhow::Ok;
use colored::Colorize;
use futures_util::AsyncBufReadExt;
use futures_util::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, LogParams};
use kube::ResourceExt;

use crate::util;

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
