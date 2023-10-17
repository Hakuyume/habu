use serde::{de, Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub(crate) struct Config {
    pub(crate) python: String,
    pub(crate) steps: Vec<Step>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct Step {
    pub(crate) index_url: Option<String>,
    #[serde(default)]
    pub(crate) extra_index_urls: Vec<String>,
    #[serde(default)]
    pub(crate) packages: HashMap<String, Package>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Package {
    Index {
        version: Option<pep440_rs::VersionSpecifiers>,
    },
    Path {
        path: PathBuf,
        editable: bool,
    },
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Inner {
            python: String,
            #[serde(default)]
            steps: Vec<Step>,

            index_url: Option<String>,
            #[serde(default)]
            extra_index_urls: Vec<String>,
            #[serde(default)]
            packages: HashMap<String, Package>,
        }

        let mut inner = Inner::deserialize(deserializer)?;
        if !inner.packages.is_empty() {
            inner.steps.push(Step {
                index_url: inner.index_url,
                extra_index_urls: inner.extra_index_urls,
                packages: inner.packages,
            })
        }
        Ok(Self {
            python: inner.python,
            steps: inner.steps,
        })
    }
}

impl<'de> Deserialize<'de> for Package {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Default, Deserialize)]
        struct Inner {
            version: Option<String>,
            path: Option<PathBuf>,
            editable: Option<bool>,
        }
        impl Inner {
            fn into<E>(self) -> Result<Package, E>
            where
                E: de::Error,
            {
                match self {
                    Self {
                        version,
                        path: None,
                        editable: None,
                    } => Ok(Package::Index {
                        version: version
                            .map(|version| version.parse())
                            .transpose()
                            .map_err(E::custom)?,
                    }),
                    Self {
                        version: None,
                        path: Some(path),
                        editable,
                    } => Ok(Package::Path {
                        path,
                        editable: editable.unwrap_or(false),
                    }),
                    _ => Err(E::custom("invalid combination of fields")),
                }
            }
        }

        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Package;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Inner {
                    version: Some(v.to_owned()),
                    ..Default::default()
                }
                .into()
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                Inner::deserialize(de::value::MapAccessDeserializer::new(map))?.into()
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests;
