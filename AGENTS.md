# AGENTS.md — Koentji

## What this is

Go API-key management service (koentji).
It runs on the RKE2 cluster in namespace `koentji` with 3 replicas, one pod per node.
PostgreSQL runs on CloudNativePG in the `database` namespace (`main` cluster, service `main-rw`).

## Release pipeline (GHCR)

- Docker image: `ghcr.io/rizquuula/koentji` (private package).
- CI/CD: `.github/workflows/release.yml`. Push a git tag (`v*.*.*` style).
  GitHub Actions builds the image, pushes `latest` plus the tag to GHCR, and opens a GitHub Release.
- Registry login uses the repo secret `CR_PAT`.

## Deployment (kube / docker)

- kube: manifests live in the `infra-control-panel` repo at `manifests/koentji/`.
  To deploy a new tag, update the image tag in the deployment manifest and apply.
  The cluster pulls this image without an imagePullSecret.
- docker: the image runs anywhere with Docker.
  Run `docker login ghcr.io` (account `rizquuula`), then `docker pull ghcr.io/rizquuula/koentji:<tag>`.
