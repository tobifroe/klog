# klog

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) 
[![Coverage Status](https://coveralls.io/repos/github/tobifroe/klog/badge.svg?branch=main)](https://coveralls.io/github/tobifroe/klog?branch=main)

klog is a tool that allows you to tail logs of multiple Kubernetes pods simultaneously.

## Features

- **Multi-pod log streaming**: Stream logs from multiple pods simultaneously with color-coded output
- **Dynamic pod discovery**: Automatically discover and stream logs from newly spawned pods
- **Multiple resource types**: Monitor Deployments, StatefulSets, DaemonSets, Jobs, and CronJobs
- **Configurable refresh**: Set custom intervals for pod discovery (default: 30 seconds)
- **Log filtering**: Filter logs to show only lines containing specific text
- **JSON log formatting**: Automatically format and colorize JSON log entries 

## Installation
### Cargo
You can build and install klog using cargo:
```bash
# Using Cargo
cargo install klog
```
### Homebrew
```bash
brew tap tobifroe/homewbrew-klog
brew install klog
```
### Nix
Klog is [packaged in nixpkgs](https://search.nixos.org/packages?channel=25.05&show=klog-rs&from=0&size=50&sort=relevance&type=packages&query=klog-rs).
```bash
nix-shell -p klog-rs
```

### Manual installation
Alternatively, grab a pre-built binary for your OS from the [releases page](https://github.com/tobifroe/klog/releases).
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
    --refresh-interval <SECONDS>      Refresh interval for discovering new pods (0 to disable) [default: 30]
-h, --help                            Print help
-V, --version                         Print version
```

## Examples

### Basic Usage
To tail logs from pods `pod1`, `pod2`, `pod3` and deployment `my-service` in the `my-namespace` namespace and follow the logs, run:

```bash
klog -n my-namespace -p pod1 pod2 pod3 -d my-service --follow
```

### Dynamic Pod Discovery
klog automatically discovers and streams logs from newly spawned pods. By default, it checks for new pods every 30 seconds:

```bash
# Monitor a deployment and automatically pick up new pods
klog -n my-namespace -d my-service --follow

# Custom refresh interval (check every 10 seconds)
klog -n my-namespace -d my-service --refresh-interval 10 --follow

# Disable automatic discovery (original behavior)
klog -n my-namespace -d my-service --refresh-interval 0 --follow
```

### Multiple Resource Types
You can monitor multiple types of Kubernetes resources simultaneously:

```bash
# Monitor deployments, statefulsets, and daemonsets
klog -n my-namespace -d app1 app2 -s db-cluster -daemonsets logging-agent --follow
```

### Filtering Logs
Filter logs to show only lines containing specific text:

```bash
# Only show log lines containing "ERROR"
klog -n my-namespace -d my-service --filter "ERROR" --follow
```

## Acknowledgements

- [Clap](https://github.com/clap-rs/clap) for argument parsing.
- [Kube](https://github.com/clux/kube-rs) for Kubernetes API interactions.
- [Tokio](https://github.com/tokio-rs/tokio) for asynchronous runtime.

