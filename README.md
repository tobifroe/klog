# klog

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/tobifroe/klog)](https://rust-reportcard.xuri.me/report/github.com/tobifroe/klog)
[![Coverage Status](https://coveralls.io/repos/github/tobifroe/klog/badge.svg?branch=main)](https://coveralls.io/github/tobifroe/klog?branch=main)

klog is a tool that allows you to tail logs of multiple Kubernetes pods simultaneously. 

## Installation
You can build and install klog using cargo:
```bash
# Using Cargo
cargo install klog
```
alternatively, grab a pre-built binary for your OS from the [releases page](https://github.com/tobifroe/klog/releases).
Curently, there are x86_64 binaries provided for Windows, MacOS and Linux.


## Usage
klog will use your current sessions kubecontext.

```bash
klog [OPTIONS] --namespace <NAMESPACE> --pods <PODS>...

# Example
klog -n my-namespace -p pod1 pod2 pod3 -f
```

### Options

```
-n, --namespace <NAMESPACE>           Namespace to use
-d, --deployments <DEPLOYMENTS>...    Deployment to log
-s, --statefulsets <STATEFULSETS>...  Statefulsets to log
    --daemonsets <DAEMONSETS>...      Daemonsets to log
    --jobs <JOBS>...                  Jobs to log
    --cronjobs <CRONJOBS>...          CronJobs to log
-p, --pods <PODS>...                  Pods to log
-f, --follow                          Follow log?
    --filter <FILTER>                 Filter [default: ]
-h, --help                            Print help
-V, --version                         Print version
```

## Example

To tail logs from pods `pod1`, `pod2`, `pod3` and deployment `my-service` in the `my-namespace` namespace and follow the logs, run:

```bash
klog -n my-namespace -p pod1 pod2 pod3 -d my-service --follow
```

## Acknowledgements

- [Clap](https://github.com/clap-rs/clap) for argument parsing.
- [Kube](https://github.com/clux/kube-rs) for Kubernetes API interactions.
- [Tokio](https://github.com/tokio-rs/tokio) for asynchronous runtime.

