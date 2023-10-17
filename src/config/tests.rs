use serde::Deserialize;

#[test]
fn test_deserialize() {
    let config = super::Config::deserialize(&serde_json::json!({
        "python": "3.12",
        "steps": [
            {
                "packages": {
                    "a": {},
                 },
            },
            {
                "index_url": "https://example.org/alpha/simple",
                "packages": {
                    "b": ">=2.0",
                    "c": "==3.0",
                 },
            },
            {
                "extra_index_urls": ["https://example.org/beta/simple"],
                "packages": {
                    "d": {"version": "<4.0"},
                },
            },
        ],
        "index_url": "https://example.org/gamma/simple",
        "extra_index_urls": ["https://example.org/delta/simple"],
        "packages": {
            "e": {"path": "path/to/e"},
            "f": {"path": "path/to/f", "editable": true},
        },
    }))
    .unwrap();

    assert_eq!(
        &config,
        &super::Config {
            python: "3.12".to_owned(),
            steps: vec![
                super::Step {
                    index_url: None,
                    extra_index_urls: Vec::new(),
                    packages: [("a".to_owned(), super::Package::Index { version: None })]
                        .into_iter()
                        .collect()
                },
                super::Step {
                    index_url: Some("https://example.org/alpha/simple".to_owned()),
                    extra_index_urls: Vec::new(),
                    packages: [
                        (
                            "b".to_owned(),
                            super::Package::Index {
                                version: Some(
                                    [pep440_rs::VersionSpecifier::new(
                                        pep440_rs::Operator::GreaterThanEqual,
                                        pep440_rs::Version::from_release(vec![2, 0]),
                                        false
                                    )
                                    .unwrap()]
                                    .into_iter()
                                    .collect()
                                )
                            }
                        ),
                        (
                            "c".to_owned(),
                            super::Package::Index {
                                version: Some(
                                    [pep440_rs::VersionSpecifier::new(
                                        pep440_rs::Operator::Equal,
                                        pep440_rs::Version::from_release(vec![3, 0]),
                                        false
                                    )
                                    .unwrap()]
                                    .into_iter()
                                    .collect()
                                )
                            }
                        )
                    ]
                    .into_iter()
                    .collect(),
                },
                super::Step {
                    index_url: None,
                    extra_index_urls: vec!["https://example.org/beta/simple".to_owned()],
                    packages: [(
                        "d".to_owned(),
                        super::Package::Index {
                            version: Some(
                                [pep440_rs::VersionSpecifier::new(
                                    pep440_rs::Operator::LessThan,
                                    pep440_rs::Version::from_release(vec![4, 0]),
                                    false
                                )
                                .unwrap()]
                                .into_iter()
                                .collect()
                            )
                        }
                    ),]
                    .into_iter()
                    .collect(),
                },
                super::Step {
                    index_url: Some("https://example.org/gamma/simple".to_owned()),
                    extra_index_urls: vec!["https://example.org/delta/simple".to_owned()],
                    packages: [
                        (
                            "e".to_owned(),
                            super::Package::Path {
                                path: "path/to/e".into(),
                                editable: false,
                            }
                        ),
                        (
                            "f".to_owned(),
                            super::Package::Path {
                                path: "path/to/f".into(),
                                editable: true,
                            }
                        )
                    ]
                    .into_iter()
                    .collect(),
                },
            ],
        },
    );
}
