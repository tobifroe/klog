use k8s_openapi::api::apps::v1::DaemonSet;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;

pub trait SpecSelector {
    fn selector(&self) -> Option<&LabelSelector>;
}

impl SpecSelector for k8s_openapi::api::apps::v1::DeploymentSpec {
    fn selector(&self) -> Option<&LabelSelector> {
        Some(&self.selector)
    }
}

impl SpecSelector for k8s_openapi::api::apps::v1::StatefulSetSpec {
    fn selector(&self) -> Option<&LabelSelector> {
        Some(&self.selector)
    }
}

impl SpecSelector for k8s_openapi::api::apps::v1::DaemonSetSpec {
    fn selector(&self) -> Option<&LabelSelector> {
        Some(&self.selector)
    }
}

impl SpecSelector for k8s_openapi::api::batch::v1::JobSpec {
    fn selector(&self) -> Option<&LabelSelector> {
        self.selector.as_ref()
    }
}

impl SpecSelector for k8s_openapi::api::batch::v1::CronJobSpec {
    fn selector(&self) -> Option<&LabelSelector> {
        self.job_template
            .spec
            .as_ref()
            .and_then(|spec| spec.selector.as_ref())
    }
}

pub trait HasSpec {
    type Spec: SpecSelector;
    fn spec(&self) -> Option<&Self::Spec>;
    fn selector(&self) -> Option<&LabelSelector>;
}

impl HasSpec for Deployment {
    type Spec = k8s_openapi::api::apps::v1::DeploymentSpec;
    fn spec(&self) -> Option<&Self::Spec> {
        self.spec.as_ref()
    }
    fn selector(&self) -> Option<&LabelSelector> {
        Some(&self.spec.as_ref().unwrap().selector)
    }
}

impl HasSpec for StatefulSet {
    type Spec = k8s_openapi::api::apps::v1::StatefulSetSpec;
    fn spec(&self) -> Option<&Self::Spec> {
        self.spec.as_ref()
    }
    fn selector(&self) -> Option<&LabelSelector> {
        Some(&self.spec.as_ref().unwrap().selector)
    }
}

impl HasSpec for DaemonSet {
    type Spec = k8s_openapi::api::apps::v1::DaemonSetSpec;
    fn spec(&self) -> Option<&Self::Spec> {
        self.spec.as_ref()
    }
    fn selector(&self) -> Option<&LabelSelector> {
        Some(&self.spec.as_ref().unwrap().selector)
    }
}

impl HasSpec for Job {
    type Spec = k8s_openapi::api::batch::v1::JobSpec;
    fn spec(&self) -> Option<&Self::Spec> {
        self.spec.as_ref()
    }
    fn selector(&self) -> Option<&LabelSelector> {
        self.spec.as_ref()?.selector.as_ref()
    }
}

impl HasSpec for CronJob {
    type Spec = k8s_openapi::api::batch::v1::CronJobSpec;
    fn spec(&self) -> Option<&Self::Spec> {
        self.spec.as_ref()
    }
    fn selector(&self) -> Option<&LabelSelector> {
        self.spec.as_ref().and_then(|spec| {
            spec.job_template
                .spec
                .as_ref() // Safely access the inner Option
                .and_then(|job_spec| job_spec.selector.as_ref()) // Safely access the selector field
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::{
        apps::v1::{
            DaemonSet, DaemonSetSpec, Deployment, DeploymentSpec, StatefulSet, StatefulSetSpec,
        },
        batch::v1::{Job, JobSpec},
    };
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;

    fn create_label_selector() -> LabelSelector {
        LabelSelector {
            match_labels: Some(
                [("key".to_string(), "value".to_string())]
                    .into_iter()
                    .collect(),
            ),
            match_expressions: None,
        }
    }

    #[test]
    fn test_spec_selector_for_deployment() {
        let selector = create_label_selector();
        let spec = DeploymentSpec {
            selector: selector.clone(),
            ..Default::default()
        };

        assert_eq!(spec.selector().unwrap(), &selector);
    }

    #[test]
    fn test_spec_selector_for_statefulset() {
        let selector = create_label_selector();
        let spec = StatefulSetSpec {
            selector: selector.clone(),
            ..Default::default()
        };

        assert_eq!(spec.selector().unwrap(), &selector);
    }

    #[test]
    fn test_spec_selector_for_daemonset() {
        let selector = create_label_selector();
        let spec = DaemonSetSpec {
            selector: selector.clone(),
            ..Default::default()
        };

        assert_eq!(spec.selector().unwrap(), &selector);
    }

    #[test]
    fn test_spec_selector_for_job() {
        let selector = create_label_selector();
        let spec = JobSpec {
            selector: Some(selector.clone()),
            ..Default::default()
        };

        assert_eq!(spec.selector().unwrap(), &selector);
    }

    #[test]
    fn test_has_spec_for_deployment() {
        let selector = create_label_selector();
        let spec = DeploymentSpec {
            selector: selector.clone(),
            ..Default::default()
        };
        let deployment = Deployment {
            spec: Some(spec),
            ..Default::default()
        };

        assert_eq!(deployment.selector().unwrap(), &selector);
    }

    #[test]
    fn test_has_spec_for_statefulset() {
        let selector = create_label_selector();
        let spec = StatefulSetSpec {
            selector: selector.clone(),
            ..Default::default()
        };
        let statefulset = StatefulSet {
            spec: Some(spec),
            ..Default::default()
        };

        assert_eq!(statefulset.selector().unwrap(), &selector);
    }

    #[test]
    fn test_has_spec_for_daemonset() {
        let selector = create_label_selector();
        let spec = DaemonSetSpec {
            selector: selector.clone(),
            ..Default::default()
        };
        let daemonset = DaemonSet {
            spec: Some(spec),
            ..Default::default()
        };

        assert_eq!(daemonset.selector().unwrap(), &selector);
    }

    #[test]
    fn test_has_spec_for_job() {
        let selector = create_label_selector();
        let spec = JobSpec {
            selector: Some(selector.clone()),
            ..Default::default()
        };
        let job = Job {
            spec: Some(spec),
            ..Default::default()
        };

        assert_eq!(job.selector().unwrap(), &selector);
    }
}
