use crate::error::{Error, Result};
use crate::models::{DataSize, ImageName, PortForward, UserName};
use serde::{Deserialize, Deserializer, de};
use std::fmt::Display;
use std::str::FromStr;

const SUPPORTED_TEMPLATE_VERSIONS: &[u32] = &[1];

/// Default values for a new instance, loaded from a user written TOML file.
///
/// Typed fields (image, memory, ports, ...) are parsed from their human friendly
/// command line form (e.g. "4G", "8000:80"), not the on-disk instance config form,
/// so a template reads the same way a `cubic create` command line does.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Template {
    pub version: Option<u32>,
    #[serde(deserialize_with = "Template::deserialize_from_str_opt")]
    pub image: Option<ImageName>,
    #[serde(deserialize_with = "Template::deserialize_from_str_opt")]
    pub user: Option<UserName>,
    pub cpus: Option<u16>,
    #[serde(deserialize_with = "Template::deserialize_from_str_opt")]
    pub memory: Option<DataSize>,
    #[serde(deserialize_with = "Template::deserialize_from_str_opt")]
    pub disk: Option<DataSize>,
    pub isolate: Option<bool>,
    #[serde(deserialize_with = "Template::deserialize_from_str_vec")]
    pub ports: Vec<PortForward>,
    pub run: Vec<String>,
}

impl Template {
    pub fn parse(content: &str) -> Result<Template> {
        let template: Template =
            toml::from_str(content).map_err(|error| Error::InvalidTemplate(error.to_string()))?;

        let version = template.version.ok_or(Error::MissingTemplateVersion)?;

        if !SUPPORTED_TEMPLATE_VERSIONS.contains(&version) {
            return Err(Error::UnsupportedTemplateVersion(version));
        }

        Ok(template)
    }

    fn deserialize_from_str_opt<'de, D, T>(
        deserializer: D,
    ) -> std::result::Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
        T::Err: Display,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|value| T::from_str(&value).map_err(de::Error::custom))
            .transpose()
    }

    fn deserialize_from_str_vec<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
        T::Err: Display,
    {
        Vec::<String>::deserialize(deserializer)?
            .iter()
            .map(|value| T::from_str(value).map_err(de::Error::custom))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_template() {
        let template = Template::parse(
            r#"
version = 1
image = "debian:trixie"
user = "john"
cpus = 4
memory = "4G"
disk = "100G"
isolate = true
ports = ["8000:80", "5353:53/udp"]
run = ["sudo apt update", "sudo apt install vim"]
"#,
        )
        .unwrap();

        assert_eq!(template.image.unwrap().get_name(), "trixie");
        assert_eq!(template.user.unwrap().as_str(), "john");
        assert_eq!(template.cpus, Some(4));
        assert_eq!(template.memory.unwrap().get_bytes(), 4 * 1024_usize.pow(3));
        assert_eq!(template.disk.unwrap().get_bytes(), 100 * 1024_usize.pow(3));
        assert_eq!(template.isolate, Some(true));
        assert_eq!(template.ports.len(), 2);
        assert_eq!(template.run, ["sudo apt update", "sudo apt install vim"]);
    }

    /// An omitted field must fall back instead of failing the whole parse.
    #[test]
    fn test_parse_leaves_omitted_fields_unset() {
        let template = Template::parse("version = 1\nimage = \"ubuntu:noble\"\n").unwrap();

        assert_eq!(template.image.unwrap().get_name(), "noble");
        assert_eq!(template.cpus, None);
        assert!(template.run.is_empty());
    }

    #[test]
    fn test_parse_rejects_an_unknown_field_and_an_invalid_value() {
        for content in [
            "version = 1\nunknown = 1\n",
            "version = 1\nports = [\"not-a-port\"]\n",
            "version = \"1\"\n",
        ] {
            assert!(matches!(
                Template::parse(content),
                Err(Error::InvalidTemplate(_))
            ));
        }
    }

    #[test]
    fn test_parse_rejects_missing_version() {
        assert!(matches!(
            Template::parse("image = \"ubuntu:noble\"\n"),
            Err(Error::MissingTemplateVersion)
        ));
    }

    #[test]
    fn test_parse_rejects_unsupported_versions() {
        for version in [0, 2, 10] {
            assert!(matches!(
                Template::parse(&format!("version = {version}\n")),
                Err(Error::UnsupportedTemplateVersion(actual)) if actual == version
            ));
        }
    }
}
