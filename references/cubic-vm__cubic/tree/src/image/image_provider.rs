use crate::models::{Arch, HashAlg};

pub trait ImageProvider {
    fn get_distro(&self) -> &str;

    fn get_base_url(&self) -> &str;
    fn find_image_names(&self, content: &str) -> Vec<String>;

    fn get_image_dir_path(&self, name: &str, arch: Arch) -> String;
    fn get_image_file_pattern(&self, name: &str, arch: Arch) -> String;

    fn get_checksum_file(&self, image_file: &str, name: &str, arch: Arch) -> String;
    fn get_checksum_alg(&self) -> HashAlg;

    /// Release version, by default the name of the release directory
    fn get_version(&self, _image_file: &str, name: &str) -> String {
        name.to_string()
    }

    /// Release codename, for the distributions that have one
    fn get_codename(&self, _name: &str) -> Option<String> {
        None
    }

    /// Newest long term release, picked from versions sorted oldest first.
    /// Distributions without long term releases keep the newest version.
    fn find_stable_version(&self, versions: &[String]) -> Option<String> {
        versions.last().cloned()
    }
}
