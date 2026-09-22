# Driver image for experiments/180-registry-fixture.sh (TODO/image.md T-0206).
#
# A shell plus a TLS-capable HTTPS client, nothing else: the zot binary, its
# config, the seeded storage and the podbox binary all arrive as mounts at
# run time, so this image carries no fixture bytes and no pin but its base.
# Base pin matches experiments/Dockerfile.target (Debian bookworm).
FROM debian:bookworm@sha256:6ebd97fa83deb272194a2cf015b3d26a4d538e9ad3a7a79d544c8af5b0a01443
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
