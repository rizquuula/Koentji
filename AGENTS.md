# AGENTS.md — Koentji

## What this is

Rust web service (Leptos SSR + actix-web) — the koentji API-key management service.
The workspace crate is `guardian`.
It runs on the RKE2 cluster in namespace `koentji` with 3 replicas, one pod per node.
PostgreSQL runs on CloudNativePG in the `database` namespace (`main` cluster, service `main-rw`).

## Release pipeline (GHCR)

- Docker image: `ghcr.io/rizquuula/koentji` (private package).
- CI/CD: `.github/workflows/release.yml`. Push a git tag (`v*.*.*` style).
  GitHub Actions builds the image, pushes `latest` plus the tag to GHCR, and opens a GitHub Release.
- Registry login uses the repo secret `CR_PAT`.
- The host has no cargo toolchain; build inside `rust:1.91-bookworm` (same image as the Dockerfile builder):
  `docker run --rm -v "$PWD":/app -w /app rust:1.91-bookworm cargo <command>`.

## Lint / audit

- `.github/workflows/lint.yml` runs `cargo audit` plus a docker build gate.
- Two advisories are ignored with justification in the workflow file:
  RUSTSEC-2023-0071 (rsa, no patch yet) and RUSTSEC-2026-0258 (h2 0.3, needs the hyper 1.x stack upgrade).

## Deployment (kube / docker)

- kube: manifests live in the `infra-control-panel` repo at `manifests/koentji/`.
  To deploy a new tag, update the image tag in the deployment manifest and apply.
  The cluster pulls this image without an imagePullSecret.
- docker: the image runs anywhere with Docker.
  Run `docker login ghcr.io` (account `rizquuula`), then `docker pull ghcr.io/rizquuula/koentji:<tag>`.
