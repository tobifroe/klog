.PHONY: all prepare_tests test cleanup_tests

all: prepare_tests test cleanup_tests

prepare_tests:
	minikube start
	kubectl apply -f tests/hack/statefulset.yaml

test:
	cargo test

cleanup_tests:
	minikube delete
