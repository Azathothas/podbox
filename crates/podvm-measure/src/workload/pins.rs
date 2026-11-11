//! The pinned inputs of the workload: the guest image and the kernel,
//! read from `pod-services` (TODO/deps.md T-1679).

/// The pin of `pod-services` (`ecr-public-registry`, fact `guest-userland`,
/// TODO/deps.md T-1679).
fn alpine_pin() -> String {
    let s =
        pod_services::get("ecr-public-registry").expect("pod-services holds ecr-public-registry");
    let host = s
        .endpoint("registry")
        .expect("ecr-public-registry has the endpoint registry")
        .host();
    let image = s
        .text("guest-userland")
        .expect("ecr-public-registry has the fact guest-userland");
    format!("{host}/{image}")
}
/// The same digest from mirror.gcr.io, for a host where public.ecr.aws
/// answers image pulls with HTTP 429 (TODO/deps.md T-1687, the cloud host
/// of 2026-11-09 and 2026-11-10). `PODVM_WORKLOAD_MIRROR=1` selects it; the
/// digest is the pin, so the image is the same.
const ALPINE_MIRROR_REF: &str = "mirror.gcr.io/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764";

/// The image reference of this run: the pin, from the mirror when
/// `PODVM_WORKLOAD_MIRROR=1`.
pub(super) fn alpine_ref() -> String {
    match std::env::var("PODVM_WORKLOAD_MIRROR").as_deref() {
        Ok("1") => ALPINE_MIRROR_REF.to_string(),
        _ => alpine_pin(),
    }
}
/// The netboot archive and the SHA-256 of its kernel, from `pod-services`
/// (`alpine-netboot`, TODO/deps.md T-1679).
fn netboot() -> &'static pod_services::Service {
    pod_services::get("alpine-netboot").expect("pod-services holds alpine-netboot")
}
pub(super) fn kurl() -> String {
    netboot()
        .endpoint("archive")
        .expect("alpine-netboot has the endpoint archive")
        .url()
}
pub(super) fn kernel_sha256() -> &'static str {
    netboot()
        .text("vmlinuz-virt-sha256")
        .expect("alpine-netboot has the fact vmlinuz-virt-sha256")
}
