//! The registered command set, checked the way Discord and the prefix parser see it.

use std::collections::HashMap;

use regex::Regex;
use serde_json::Value;

/// Discord's rule for command and option names (lowercase where the script has case).
const NAME: &str = r"^[-_\p{Ll}\p{Lo}\p{N}]{1,32}$";

fn slash_definitions() -> Vec<Value> {
    lyra::commands()
        .iter()
        .filter_map(|command| command.create_as_slash_command())
        .map(|definition| serde_json::to_value(definition).unwrap())
        .collect()
}

#[test]
fn names_and_aliases_do_not_collide() {
    // A prefix invocation resolves by name or alias, so all of them must be unique.
    let mut seen: HashMap<String, String> = HashMap::new();
    for command in lyra::commands() {
        let mut words: Vec<_> = std::iter::once(&command.name)
            .chain(command.aliases.iter())
            .map(|word| word.to_string())
            .collect();
        // Listing a command's own name as an alias is redundant but harmless.
        words.sort();
        words.dedup();
        for word in words {
            if let Some(owner) = seen.insert(word.clone(), command.name.to_string()) {
                panic!("`{word}` is used by both `{owner}` and `{}`", command.name);
            }
        }
    }
}

#[test]
fn every_command_has_a_category_and_description_for_help() {
    for command in lyra::commands() {
        if command.owners_only || command.hide_in_help {
            continue; // not listed by `help`
        }
        assert!(
            command.category.is_some(),
            "`{}` has no category",
            command.name
        );
        assert!(
            command
                .description
                .as_deref()
                .is_some_and(|d| !d.is_empty()),
            "`{}` has no description",
            command.name
        );
    }
}

#[test]
fn music_commands_only_run_in_guilds() {
    // They all need a voice channel and the guild's queue.
    for command in lyra::commands() {
        if command.category.as_deref() == Some("Music") {
            assert!(command.guild_only, "`{}` is not guild_only", command.name);
        }
    }
}

#[test]
fn slash_definitions_satisfy_discord_limits() {
    let name = Regex::new(NAME).unwrap();
    let definitions = slash_definitions();
    assert!(!definitions.is_empty());

    for command in &definitions {
        let command_name = command["name"].as_str().unwrap();
        assert!(
            name.is_match(command_name),
            "bad command name {command_name:?}"
        );
        let description = command["description"].as_str().unwrap_or_default();
        assert!(
            (1..=100).contains(&description.chars().count()),
            "`{command_name}` description must be 1-100 characters: {description:?}"
        );

        let options = command["options"].as_array().cloned().unwrap_or_default();
        assert!(
            options.len() <= 25,
            "`{command_name}` has more than 25 options"
        );

        let mut optional_seen = false;
        for option in &options {
            let option_name = option["name"].as_str().unwrap();
            assert!(
                name.is_match(option_name),
                "`{command_name}` option {option_name:?} is not a valid name"
            );
            let option_description = option["description"].as_str().unwrap_or_default();
            assert!(
                (1..=100).contains(&option_description.chars().count()),
                "`{command_name}` option `{option_name}` description must be 1-100 characters"
            );

            // Discord rejects required options after optional ones.
            let required = option["required"].as_bool().unwrap_or(false);
            assert!(
                !(required && optional_seen),
                "`{command_name}`: required option `{option_name}` follows an optional one"
            );
            optional_seen |= !required;
        }
    }
}

#[test]
fn every_command_except_owner_tools_is_a_slash_command() {
    let slash: Vec<_> = slash_definitions()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();

    for command in lyra::commands() {
        if command.owners_only || command.hide_in_help {
            continue;
        }
        assert!(
            slash.contains(&command.name.to_string()),
            "`{}` has no slash variant",
            command.name
        );
    }
}

#[test]
fn queue_control_commands_are_registered() {
    let names: Vec<_> = lyra::commands()
        .iter()
        .map(|c| c.name.to_string())
        .collect();
    for expected in [
        "play",
        "forceplay",
        "jump",
        "repeat",
        "seek",
        "skip",
        "queue",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "`{expected}` is not registered"
        );
    }
}
