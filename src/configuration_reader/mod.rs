use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::{thread, time};

use crate::FileDescription;
use crate::StepInChain;

#[derive(Debug)]
pub struct Configuration {
    pub files: Vec<FileDescription>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonConfiguration {
    aliases: Option<Vec<JsonAliases>>,
    from_aliases: Option<Vec<JsonAliases>>,
    to_aliases: Option<Vec<JsonAliases>>,
    files: Vec<JsonFileDescription>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonFileDescription {
    from: String,
    through: String,
    to: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonAliases {
    name: String,
    replacement: String,
}

fn simplify(to_simplify: &str) -> String {
    let mut s = String::from(to_simplify.clone());
    while (s.contains("\\\\")) {
        s = s.replace("\\\\", "\\");
    }
    s
}

fn parse_configuration_from_string(
    json_string: &str,
    step_in_chain: StepInChain,
) -> Result<(Configuration, Vec<FileDescription>), Error> {
    let json_configuration: JsonConfiguration = serde_json::from_str(&json_string)?;

    let mut configuration: Configuration = Configuration { files: Vec::new() };

    let mut failed_vec: Vec<FileDescription> = Vec::new();
    let mut alias_map: HashMap<String, String> = HashMap::new();

    if json_configuration.aliases.is_some() {
        json_configuration
            .aliases
            .unwrap()
            .iter()
            .for_each(|alias| {
                alias_map.insert(alias.name.to_owned(), alias.replacement.to_owned());
            });
    }

    // Insert more aliases based on step in copy chain.
    match step_in_chain {
        StepInChain::Start => {
            if json_configuration.from_aliases.is_some() {
                json_configuration
                    .from_aliases
                    .unwrap()
                    .iter()
                    .for_each(|alias| {
                        alias_map.insert(alias.name.to_owned(), alias.replacement.to_owned());
                    });
            }
        }
        StepInChain::End => {
            if json_configuration.to_aliases.is_some() {
                json_configuration
                    .to_aliases
                    .unwrap()
                    .iter()
                    .for_each(|alias| {
                        alias_map.insert(alias.name.to_owned(), alias.replacement.to_owned());
                    });
            }
        }
    }

    json_configuration.files.iter().for_each(|f| {
        let mut from: String;
        let mut to: String;
        match step_in_chain {
            StepInChain::Start => {
                from = f.from.to_owned();
                to = f.through.to_owned();
            }
            StepInChain::End => {
                from = f.through.to_owned();
                to = f.to.to_owned();
            }
        }

        from = simplify(&from);
        to = simplify(&to);

        alias_map.iter().for_each(|am| {
            let alias: &str = &("@".to_owned() + am.0);
            println!("alias: {:?}", alias);
            let replacement: &str = am.1;
            println!("replacement: {:?}", alias);
            from = from.replace(alias, replacement);
            to = to.replace(alias, replacement);
        });
        configuration.files.push(FileDescription {
            from: PathBuf::from(from),
            to: PathBuf::from(to),
        });
    });

    Ok((configuration, failed_vec))
}

pub fn parse_configuration(
    path: &str,
    step_in_chain: StepInChain,
) -> Result<(Configuration, Vec<FileDescription>), Error> {
    let mut file = File::open(path)?;
    let mut contents: String = String::new();
    file.read_to_string(&mut contents)?;
    parse_configuration_from_string(&contents, step_in_chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_aliases() {
        let parse_result = parse_configuration_from_string(
            r#"{
                "aliases": [
                    {
                        "name": "project-name",
                        "replacement": "autocopier"
                    }
                ],
                "files": [
                    {
                        "from": ".\\@project-name\\configuration.json",
                        "through": ".\\@project-name\\configuration_copy_middle.json",
                        "to": ".\\@project-name\\configuration_copy.json"
                    }
                ]
            }"#,
            StepInChain::Start,
        );
        assert!(parse_result.is_ok());
        let (configuration, unparsed) = parse_result.unwrap();
        assert_eq!(configuration.files.len(), 1);
        assert_eq!(
            PathBuf::from(".\\autocopier\\configuration.json"),
            configuration.files.get(0).unwrap().from
        );
        assert_eq!(
            PathBuf::from(".\\autocopier\\configuration_copy_middle.json"),
            configuration.files.get(0).unwrap().to
        );
    }

    #[test]
    fn test_from_aliases() {
        let parse_result = parse_configuration_from_string(
            r#"{
                "aliases": [

                ],
                "from_aliases": [
                    {
                        "name": "project-name",
                        "replacement": "autocopier"
                    }
                ],
                "to_aliases": [
                    {
                        "name": "project-name",
                        "replacement": "wrongcopier"
                    }
                ],
                "files": [
                    {
                        "from": ".\\@project-name\\configuration.json",
                        "through": ".\\@project-name\\configuration_copy_middle.json",
                        "to": ".\\@project-name\\configuration_copy.json"
                    }
                ]
            }"#,
            StepInChain::Start,
        );
        assert!(parse_result.is_ok());
        let (configuration, unparsed) = parse_result.unwrap();
        assert_eq!(configuration.files.len(), 1);
        assert_eq!(
            PathBuf::from(".\\autocopier\\configuration.json"),
            configuration.files.get(0).unwrap().from
        );
        // Assert the to (which is through in the start )
        assert_eq!(
            PathBuf::from(".\\autocopier\\configuration_copy_middle.json"),
            configuration.files.get(0).unwrap().to
        );
    }

    #[test]
    fn test_to_aliases() {
        let parse_result = parse_configuration_from_string(
            r#"{
                "from_aliases": [
                    {
                        "name": "project-name",
                        "replacement": "autocopier"
                    }
                ],
                "to_aliases": [
                    {
                        "name": "project-name",
                        "replacement": "tocopier"
                    }
                ],
                "files": [
                    {
                        "from": ".\\@project-name\\configuration.json",
                        "through": ".\\@project-name\\configuration_copy_middle.json",
                        "to": ".\\@project-name\\configuration_copy.json"
                    }
                ]
            }"#,
            StepInChain::End,
        );
        assert!(parse_result.is_ok());
        let (configuration, unparsed) = parse_result.unwrap();
        assert_eq!(configuration.files.len(), 1);
        assert_eq!(
            PathBuf::from(".\\tocopier\\configuration_copy_middle.json"),
            configuration.files.get(0).unwrap().from
        );
        assert_eq!(
            PathBuf::from(".\\tocopier\\configuration_copy.json"),
            configuration.files.get(0).unwrap().to
        );
    }

    #[test]
    fn test_simplify() {
        let start_string = "Test\\\\\\Extra\\\\More\\";
        let result = simplify(start_string);
        let expected = "Test\\Extra\\More\\";
        assert_eq!(expected, &result);
    }
}
