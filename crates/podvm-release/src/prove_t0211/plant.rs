// The place of the two T-0211 mutations. `Lock` is
// `pod_supervise::lock::Lock` since podcrates T-060, so the two defences are
// in the podcrates checkout that cargo builds, not in this tree. The proof
// copies that checkout, plants the mutation in the copy, and points each
// podcrates package of `Cargo.lock` at the copy with `--config patch`. Each
// package moves together: a patch of `pod-supervise` alone would give it a
// second `pod-hostprobe`, with a second fork-close registry.

use super::*;

/// The git source of the podcrates packages, as `Cargo.lock` names it.
const PODCRATES: &str = "https://github.com/PodUtils/podcrates";

/// The root of the podcrates checkout that cargo builds, from
/// `cargo metadata` (the manifest path of `pod-supervise`).
pub(super) fn podcrates_checkout(root: &Path) -> Option<PathBuf> {
    let (ok, text) = run_to_string(
        "cargo",
        &["metadata", "--format-version", "1", "--locked"],
        root,
        &[],
        600,
    )?;
    if !ok {
        return None;
    }
    let end = text.find("/crates/pod-supervise/Cargo.toml\"")?;
    let start = text[..end].rfind('"')? + 1;
    let dir = PathBuf::from(&text[start..end]);
    dir.is_dir().then_some(dir)
}

/// The names of the packages that `Cargo.lock` takes from podcrates.
pub(super) fn podcrates_packages(lock: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut name = None;
    for line in lock.lines() {
        if line == "[[package]]" {
            name = None;
        } else if let Some(rest) = line.strip_prefix("name = \"") {
            name = rest.strip_suffix('"').map(str::to_string);
        } else if let Some(rest) = line.strip_prefix("source = \"git+")
            && rest.starts_with(PODCRATES)
            && let Some(n) = name.take()
        {
            names.push(n);
        }
    }
    names
}

/// The cargo arguments that point each package of `names` at the crates of
/// `copy`, or `None` when the copy does not have one of them.
pub(super) fn patch_args(copy: &Path, names: &[String]) -> Option<Vec<String>> {
    let mut args = Vec::new();
    for name in names {
        let dir = copy.join("crates").join(name);
        if !dir.join("Cargo.toml").is_file() {
            return None;
        }
        args.push("--config".to_string());
        args.push(format!(
            "patch.\"{PODCRATES}\".{name}.path=\"{}\"",
            dir.display()
        ));
    }
    Some(args)
}

/// A copy of the podcrates checkout under `work`, and the patch arguments
/// for it.
pub(super) fn podcrates_copy(root: &Path, work: &Path) -> Option<(PathBuf, Vec<String>)> {
    let checkout = podcrates_checkout(root)?;
    let copy = work.join("podcrates");
    let ok = run_status(
        "cp",
        &["-a", &checkout.to_string_lossy(), &copy.to_string_lossy()],
        root,
        &[],
        300,
    )?;
    if !ok {
        return None;
    }
    let lock = fs::read_to_string(root.join("Cargo.lock")).ok()?;
    let names = podcrates_packages(&lock);
    if !names.iter().any(|n| n == "pod-supervise") {
        return None;
    }
    let args = patch_args(&copy, &names)?;
    Some((copy, args))
}
