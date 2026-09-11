use crate::error::Result;
use crate::image::{ImageFactory, ImageFetcher, ImageStore};
use crate::models::{Environment, Image, ImageName};
use crate::platform::System;
use crate::view::{Console, Spinner};
use std::path::Path;
use std::sync::Arc;

pub async fn fetch_image_list(
    console: &Arc<Console>,
    system: &dyn System,
    env: &Environment,
) -> Vec<Image> {
    let _spinner = Spinner::new(Arc::clone(console), "Fetching image list".to_string());
    ImageFactory::new(system, env)
        .get_all_images(console)
        .await
        .unwrap_or_default()
}

pub async fn fetch_image_info(
    console: &Arc<Console>,
    system: &dyn System,
    env: &Environment,
    image: &ImageName,
) -> Result<Image> {
    let (distro, name) = (image.get_distro(), image.get_name());
    let text = format!("Looking up image {distro}:{name}");
    let _spinner = Spinner::new(Arc::clone(console), text);
    ImageFactory::new(system, env)
        .find_image(console, image)
        .await
}

pub async fn fetch_image(
    console: &Arc<Console>,
    system: &dyn System,
    env: &Environment,
    image: &Image,
) -> Result<()> {
    if !ImageStore::new().exists(system, env, image) {
        system.create_writable_dir(Path::new(&env.get_image_dir()))?;
        ImageFetcher::new()
            .fetch(
                console,
                system,
                image,
                Path::new(&env.get_image_file(&image.to_file_name())),
            )
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Arch, HashAlg, UserName};
    use crate::platform::SystemMock;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_fetch_image_skips_cached_image() {
        let system = SystemMock::new().add_file("images/debian_bookworm_amd64", b"");
        let console = &Console::new(Arc::new(SystemMock::new()));
        let env = Environment::new(
            UserName::from_str("cubic").unwrap(),
            String::new(),
            String::new(),
        );
        let image = Image {
            distro: "debian".to_string(),
            version: "12".to_string(),
            codename: Some("bookworm".to_string()),
            tags: Vec::new(),
            arch: Arch::AMD64,
            image_url: String::new(),
            checksum_url: String::new(),
            hash_alg: HashAlg::Sha512,
            size: None,
        };

        // A cached image must return without touching the image directory
        // or the network.
        fetch_image(console, &system, &env, &image).await.unwrap();
    }
}
