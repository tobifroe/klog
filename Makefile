.PHONY: all prepare_tests test cleanup_tests

all: prepare_tests test cleanup_tests

prepare_tests:
	minikube start
	kubectl apply -f tests/hack/statefulset.yaml
	kubectl apply -f tests/hack/daemonset.yaml

test:
	cargo test --features integration-tests

cleanup_tests:
	minikube delete
