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
        self.selector()
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
       self.spec.unwrap().selector()
    }
}