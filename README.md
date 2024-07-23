# klog

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

klog is a tool that allows you to tail logs of multiple Kubernetes pods simultaneously. 

## Installation
TBD.

```bash
# Using Cargo
cargo install klog
```

## Usage

```bash
klog [OPTIONS] --namespace <NAMESPACE> --pods <PODS>...

# Example
klog -n my-namespace -p pod1 pod2 pod3 -f
```

### Options

```
-n, --namespace <NAMESPACE>    Namespace to use
-p, --pods <PODS>...           Pods to log
-f, --follow                   Follow log?
```

## Example

To tail logs from pods `pod1`, `pod2`, and `pod3` in the `my-namespace` namespace and follow the logs, run:

```bash
klog -n my-namespace -p pod1 pod2 pod3 --follow
```

## Acknowledgements

- [Clap](https://github.com/clap-rs/clap) for argument parsing.
- [Kube](https://github.com/clux/kube-rs) for Kubernetes API interactions.
- [Tokio](https://github.com/tokio-rs/tokio) for asynchronous runtime.

