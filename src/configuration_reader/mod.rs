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

fn contains_multiple(potential_multiple: &str) -> bool {
    potential_multiple.contains("{") && potential_multiple.contains("}")
}

fn extract_multiple(multiples: &str) -> Vec<String> {
    if !contains_multiple(multiples) {
        return vec![String::from(multiples)];
    }

    // we can safely unwrap because contains multiple checks the presence of these.
    let start_index = multiples.find('{').unwrap();
    let end_index = multiples.find('}').unwrap();

    // We will want to reuse this part and concatenate to it.
    let first_part = &multiples[0..start_index];
    let last_part = &multiples[end_index + 1..];

    let substr = &multiples[start_index + 1..end_index];

    let mut result_vector: Vec<String> = Vec::new();

    for mult_split in substr.split(",") {
        for last_part_m in extract_multiple(last_part) {
            result_vector.push(String::from(first_part) + mult_split + &last_part_m);
        }
    }

    result_vector
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

        alias_map.iter().for_each(|am| {
            let alias: &str = &("@".to_owned() + am.0);
            let replacement: &str = am.1;
            from = from.replace(alias, replacement);
            to = to.replace(alias, replacement);
        });
        // Check if multiple subsets are in there

        if (contains_multiple(&from)) {
            let multiple_from = extract_multiple(&from);
            let multiple_to = extract_multiple(&to);

            for (f, t) in multiple_from.iter().zip(multiple_to.iter()) {
                configuration.files.push(FileDescription {
                    from: PathBuf::from(simplify(f)),
                    to: PathBuf::from(simplify(t)),
                });
            }
        } else {
            configuration.files.push(FileDescription {
                from: PathBuf::from(simplify(&from)),
                to: PathBuf::from(simplify(&to)),
            });
        }
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
    fn test_multiple() {
        let parse_result = parse_configuration_from_string(
            r#"{
                "files": [
                    {
                        "from": "\\test\\executable.{exe,pdb}",
                        "through": "\\othertest\\executable.{exe,pdb}",
                        "to": "\\moreothertest\\executable.{exe,pdb}"
                    }
                ]
            }"#,
            StepInChain::Start,
        );
        assert!(parse_result.is_ok());
        let (configuration, unparsed) = parse_result.unwrap();
        assert_eq!(configuration.files.len(), 2);
        assert_eq!(
            PathBuf::from("\\test\\executable.exe"),
            configuration.files.get(0).unwrap().from
        );
        assert_eq!(
            PathBuf::from("\\othertest\\executable.exe"),
            configuration.files.get(0).unwrap().to
        );
        assert_eq!(
            PathBuf::from("\\test\\executable.pdb"),
            configuration.files.get(1).unwrap().from
        );
        assert_eq!(
            PathBuf::from("\\othertest\\executable.pdb"),
            configuration.files.get(1).unwrap().to
        );
    }

    #[test]
    fn test_simplify() {
        let start_string = "Test\\\\\\Extra\\\\More\\";
        let result = simplify(start_string);
        let expected = "Test\\Extra\\More\\";
        assert_eq!(expected, &result);
    }

    #[test]
    fn test_simply_configuration() {
        let parse_result = parse_configuration_from_string(
            r#"{
                "aliases": [
                    {
                        "name": "shared",
                        "replacement": "\\FpShare\\autocopier\\Files\\"
                    },
                    {
                        "name": "exedotnet",
                        "replacement": "C:\\exedotnet\\"
                    }
                ],
                "from_aliases": [
                    {
                        "name": "drive",
                        "replacement": "C:"
                    },
                    {
                        "name": "workspace",
                        "replacement": "C:\\workspaces\\GroupFuelPos\\git-FuelPos_53.90.9999999_stable\\"
                    },
                    {
                        "name": "DatabaseServer",
                        "replacement": "\\Common\\DatabaseServer\\Server\\bin\\Debug\\Framework\\"
                    }
                ],
                "to_aliases": [
                    {
                        "name": "drive",
                        "replacement": "Z:"
                    }
                ],
                "files": [
                    {
                        "from": "@workspace\\@DatabaseServer\\DatabaseServer.{exe,pdb}",
                        "through": "@drive\\@shared\\DatabaseServer\\DatabaseServer.{exe,pdb}",
                        "to": "@exedotnet\\DatabaseServer\\DatabaseServer.{exe,pdb}"
                    }
                ]
            }"#,
            StepInChain::Start,
        );
        assert!(parse_result.is_ok());
        let (configuration, unparsed) = parse_result.unwrap();
        assert_eq!(configuration.files.len(), 2);
        assert_eq!(
            "C:\\workspaces\\GroupFuelPos\\git-FuelPos_53.90.9999999_stable\\Common\\DatabaseServer\\Server\\bin\\Debug\\Framework\\DatabaseServer.exe",
            configuration.files.get(0).unwrap().from.to_string_lossy()
        );
    }

    #[test]
    fn test_contains_multiple() {
        let multiple_str = "Th{is, at}";
        assert!(contains_multiple(multiple_str));

        let non_multiple_str = "This";
        assert!(!contains_multiple(non_multiple_str));
    }

    #[test]
    fn test_extract_multiples_single() {
        let multiple_str = "Th{is,at}";
        let result = extract_multiple(multiple_str);

        let this = String::from("This");
        let that = String::from("That");

        assert!(result.contains(&this));
        assert!(result.contains(&that));
    }

    #[test]
    fn test_extract_multiples_multiple() {
        let multiple_str = "Th{is,at} and th{at,is}";
        let result = extract_multiple(multiple_str);

        let this_and_this = String::from("This and this");
        let this_and_that = String::from("This and that");
        let that_and_that = String::from("That and that");
        let that_and_this = String::from("That and this");

        assert!(result.contains(&this_and_that));
        assert!(result.contains(&this_and_this));
        assert!(result.contains(&that_and_that));
        assert!(result.contains(&that_and_this));
    }
}
