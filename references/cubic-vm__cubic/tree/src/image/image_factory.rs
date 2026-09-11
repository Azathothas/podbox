use crate::error::{Error, Result};
use crate::image::{self, ImageCache};
use crate::models::{Arch, Environment, Image, ImageName};
use crate::platform::System;
use crate::util;
use crate::view::Console;
use crate::web::WebClient;
use std::path::Path;
use std::sync::Arc;

const IMAGE_PROVIDERS: &[&dyn image::ImageProvider] = &[
    &image::AlmaLinuxImageProvider {},
    &image::ArchLinuxImageProvider {},
    &image::DebianImageProvider {},
    &image::FedoraImageProvider {},
    &image::GentooImageProvider {},
    &image::OpenSuseImageProvider {},
    &image::RockyLinuxImageProvider {},
    &image::UbuntuImageProvider {},
];

pub struct ImageFactory<'a> {
    env: Environment,
    system: &'a dyn System,
}

impl<'a> ImageFactory<'a> {
    pub fn new(system: &'a dyn System, env: &Environment) -> Self {
        Self {
            env: env.clone(),
            system,
        }
    }

    fn filter_arch(filter: Option<ImageName>) -> Vec<Arch> {
        let mut arches = vec![Arch::AMD64, Arch::ARM64];

        if let Some(filter) = filter {
            arches.retain(|a| filter.get_arch() == *a);
        }

        arches
    }

    async fn get_images_from_provider_name_arch(
        console: &Arc<Console>,
        web: &mut WebClient,
        image_provider: &dyn image::ImageProvider,
        name: &str,
        arch: Arch,
    ) -> Option<Image> {
        let image_dir_url = format!(
            "{}{}",
            image_provider.get_base_url(),
            &image_provider.get_image_dir_path(name, arch)
        );
        console.debug(&format!(
            "Fetching image directory listing '{image_dir_url}'"
        ));
        let image_content = match web.download_content(&image_dir_url).await {
            Ok(content) => content,
            Err(e) => {
                console.debug(&format!(
                    "Cannot fetch image directory listing '{image_dir_url}' ({e})"
                ));
                return None;
            }
        };

        let image_file = util::find_and_extract(
            &format!(
                "href=\"\\.?/?({})\"",
                image_provider.get_image_file_pattern(name, arch)
            ),
            &image_content,
        );
        let image_file = image_file.first();

        if let Some(image_file) = image_file {
            let image_url = format!("{image_dir_url}{image_file}");
            web.get_file_size(&image_url)
                .await
                .ok()
                .and_then(|size| size)
                .map(|size| Image {
                    distro: image_provider.get_distro().to_string(),
                    version: image_provider.get_version(image_file, name),
                    codename: image_provider.get_codename(name),
                    tags: Vec::new(),
                    arch,
                    image_url,
                    checksum_url: format!(
                        "{image_dir_url}{}",
                        image_provider.get_checksum_file(image_file, name, arch)
                    ),
                    hash_alg: image_provider.get_checksum_alg(),
                    size: Some(size),
                })
        } else {
            None
        }
    }

    /// Derive the stable and latest tag of every distro and arch. Tags follow
    /// from the version list, so they are never stored and never go stale.
    fn tag_images(mut images: Vec<Image>) -> Vec<Image> {
        images.sort();

        for image_provider in IMAGE_PROVIDERS {
            for arch in [Arch::AMD64, Arch::ARM64] {
                let is_in_group = |image: &Image| {
                    image.distro == image_provider.get_distro() && image.arch == arch
                };
                let versions = images
                    .iter()
                    .filter(|image| is_in_group(image))
                    .map(|image| image.get_version().to_string())
                    .collect::<Vec<_>>();
                let latest = versions.last().cloned();
                let stable = image_provider.find_stable_version(&versions);

                for image in images.iter_mut().filter(|image| is_in_group(image)) {
                    if stable.as_deref() == Some(image.get_version()) {
                        image.tags.push(Image::STABLE_TAG.into());
                    }
                    if latest.as_deref() == Some(image.get_version()) {
                        image.tags.push(Image::LATEST_TAG.into());
                    }
                }
            }
        }

        images
    }

    async fn get_images_from_provider(
        console: &Arc<Console>,
        web: &mut WebClient,
        image_provider: &dyn image::ImageProvider,
        filter: Option<ImageName>,
    ) -> Vec<Image> {
        let Ok(content) = web.download_content(image_provider.get_base_url()).await else {
            return Vec::new();
        };

        let mut images = Vec::new();
        for name in image_provider.find_image_names(&content) {
            for arch in Self::filter_arch(filter.clone()) {
                if let Some(image) = Self::get_images_from_provider_name_arch(
                    console,
                    web,
                    image_provider,
                    &name,
                    arch,
                )
                .await
                {
                    images.push(image);
                }
            }
        }
        images
    }

    async fn get_images(
        console: &Arc<Console>,
        web: &mut WebClient,
        filter: Option<ImageName>,
    ) -> Vec<Image> {
        let mut images = Vec::new();
        for provider in IMAGE_PROVIDERS {
            if filter.is_none() || filter.as_ref().unwrap().get_distro() == provider.get_distro() {
                images.extend(
                    Self::get_images_from_provider(console, web, *provider, filter.clone()).await,
                );
            }
        }
        images
    }

    fn find_matching_image(images: &[Image], filter: &ImageName) -> Option<Image> {
        images
            .iter()
            .find(|image| {
                image.distro == filter.get_distro()
                    && image.arch == filter.get_arch()
                    && image.has_name(filter.get_name())
            })
            .cloned()
    }

    async fn read_images(
        &self,
        console: &Arc<Console>,
        filter: Option<ImageName>,
    ) -> Result<Vec<Image>> {
        // Read cache
        let cache =
            ImageCache::read_from_file(self.system, Path::new(&self.env.get_image_cache_file()));

        // Use cache if valid
        let images = if let Some(cache) = &cache
            && cache.is_valid()
        {
            console.debug("Using cached image list");
            cache.images.clone()
        } else {
            // Fetch image info
            console.debug("Image cache missing or stale, fetching image list from providers");
            let images = Self::get_images(console, &mut WebClient::new()?, filter.clone()).await;

            // Return cache if fetching failed
            if images.is_empty()
                && let Some(cache) = &cache
            {
                console.debug("Fetching image list failed, falling back to stale cache");
                cache.images.clone()
            } else {
                // Write cache
                if filter.is_none() {
                    ImageCache::new(images.clone())
                        .write_to_file(self.system, Path::new(&self.env.get_image_cache_file()));
                }
                images
            }
        };

        let images = Self::tag_images(images);

        Ok(match &filter {
            Some(name) => Self::find_matching_image(&images, name)
                .into_iter()
                .collect(),
            None => images,
        })
    }

    pub async fn get_all_images(&self, console: &Arc<Console>) -> Result<Vec<Image>> {
        self.read_images(console, None).await
    }

    pub async fn find_image(&self, console: &Arc<Console>, name: &ImageName) -> Result<Image> {
        self.read_images(console, Some(name.clone()))
            .await
            .and_then(|images| {
                images
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::UnknownImage(name.to_string()))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HashAlg;
    use std::str::FromStr;

    fn build_image(distro: &str, version: &str, codename: Option<&str>, arch: Arch) -> Image {
        Image {
            distro: distro.to_string(),
            version: version.to_string(),
            codename: codename.map(str::to_string),
            tags: Vec::new(),
            arch,
            image_url: "image_url".to_string(),
            checksum_url: "checksum_url".to_string(),
            hash_alg: HashAlg::Sha256,
            size: None,
        }
    }

    #[test]
    fn test_tag_images_tags_the_newest_and_the_newest_lts() {
        let images = vec![
            build_image("ubuntu", "24.04", Some("noble"), Arch::AMD64),
            build_image("ubuntu", "25.10", Some("questing"), Arch::AMD64),
            build_image("ubuntu", "25.04", Some("plucky"), Arch::AMD64),
        ];

        let images = ImageFactory::tag_images(images);

        assert_eq!(images[0].get_tags(), "noble, stable");
        assert_eq!(images[1].get_tags(), "plucky");
        assert_eq!(images[2].get_tags(), "questing, latest");
    }

    #[test]
    fn test_tag_images_tags_each_arch_on_its_own() {
        let images = vec![
            build_image("ubuntu", "24.04", Some("noble"), Arch::AMD64),
            build_image("ubuntu", "26.04", Some("resolute"), Arch::AMD64),
            build_image("ubuntu", "24.04", Some("noble"), Arch::ARM64),
        ];

        let images = ImageFactory::tag_images(images);
        let find_tags = |version: &str, arch: Arch| {
            images
                .iter()
                .find(|image| image.get_version() == version && image.arch == arch)
                .map(|image| image.get_tags())
                .unwrap()
        };

        assert_eq!(find_tags("24.04", Arch::AMD64), "noble");
        assert_eq!(find_tags("26.04", Arch::AMD64), "resolute, stable, latest");
        assert_eq!(find_tags("24.04", Arch::ARM64), "noble, stable, latest");
    }

    #[test]
    fn test_tag_images_tags_a_rolling_release() {
        let images = vec![build_image("archlinux", Image::ROLLING, None, Arch::AMD64)];

        let images = ImageFactory::tag_images(images);

        assert_eq!(images[0].get_image_name(), "archlinux:rolling");
        assert_eq!(images[0].get_tags(), "stable, latest");
    }

    #[test]
    fn test_find_matching_image_matches_distro_arch_and_name() {
        let images = vec![
            build_image("almalinux", "9", None, Arch::AMD64),
            build_image("debian", "12", Some("bookworm"), Arch::AMD64),
        ];
        let filter = ImageName::from_str("debian:bookworm:amd64").unwrap();

        let found = ImageFactory::find_matching_image(&images, &filter);

        assert_eq!(found, Some(images[1].clone()));
    }

    #[test]
    fn test_find_matching_image_returns_none_on_a_mismatch() {
        let images = vec![build_image("debian", "12", Some("bookworm"), Arch::AMD64)];

        for name in [
            "ubuntu:bookworm:amd64",
            "debian:bookworm:arm64",
            "debian:bullseye:amd64",
        ] {
            let filter = ImageName::from_str(name).unwrap();

            assert_eq!(ImageFactory::find_matching_image(&images, &filter), None);
        }
    }

    #[test]
    fn test_filter_arch_keeps_the_filtered_arch_only() {
        let filter = ImageName::from_str("debian:bookworm:arm64").unwrap();

        assert_eq!(
            ImageFactory::filter_arch(None),
            vec![Arch::AMD64, Arch::ARM64]
        );
        assert_eq!(ImageFactory::filter_arch(Some(filter)), vec![Arch::ARM64]);
    }
}
