//! The command tree must build, and every completion script must generate.
//!
//! clap validates the tree at build time and panics on a bad argument
//! definition, so these run on a cold install where nothing else would.

use clap::CommandFactory;

#[path = "../src/commands/mod.rs"]
mod cli;

#[test]
fn the_command_tree_is_valid() {
    cli::Cli::command().debug_assert();
}

#[test]
fn completions_generate_for_every_shell() {
    for shell in [cli::Shell::Bash, cli::Shell::Zsh, cli::Shell::Fish] {
        let mut out = Vec::new();
        clap_complete::generate(
            clap_complete::Shell::from(shell),
            &mut cli::Cli::command(),
            "penguinwave-cli",
            &mut out,
        );
        assert!(!out.is_empty(), "{shell:?} produced nothing");
    }
}

#[test]
fn help_text_carries_no_implementation_notes() {
    // Doc comments become user-facing help, so rationale belongs in a plain
    // comment instead.
    let mut out = Vec::new();
    cli::Cli::command().write_long_help(&mut out).unwrap();
    let mut all = String::from_utf8(out).unwrap();

    for sub in cli::Cli::command().get_subcommands_mut() {
        let mut buf = Vec::new();
        sub.write_long_help(&mut buf).unwrap();
        all.push_str(&String::from_utf8(buf).unwrap());
    }

    for leak in ["clap", "panics", "unreachable", "TODO"] {
        assert!(!all.contains(leak), "help text mentions {leak:?}");
    }
}
