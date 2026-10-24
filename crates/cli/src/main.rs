//! `sonata`: lets agents in Sonata terminals see their links and talk to linked terminals.
//! It forwards each command to the running app over the private socket in `SONATA_SOCKET`;
//! the app does all the work and every permission check.

mod client;
mod hook;
mod run;

use std::io::Read;
use std::process::ExitCode;

pub const EXIT_UNREACHABLE: u8 = 6;
pub const EXIT_USAGE: u8 = 2;

const HELP: &str = "sonata — talk to the terminals and notes linked to this one.

Everyone:
  sonata list                                  who you are and what you are linked to
  sonata ask <name>[,<name>…] \"<message>\"      send a message and wait for the answer
  sonata ask --all \"<message>\"                 ask every linked terminal at once
  sonata ask <name> --keys \"down enter\"        send keys (enter tab esc up down left right space backspace ctrl-c ctrl-d)
  sonata check <name>                          show a linked terminal's screen
  sonata reply \"<message>\"                     answer the question you were asked (or --stdin)
  sonata note read <note>
  sonata note write <note> \"<text>\" [--append] (or --stdin)
  sonata notify \"<message>\"                    macOS notification for the user
  sonata browser open|read|click|fill|press|screenshot|back|forward|reload <browser> …
  sonata lane status                           your lane: branch, changed files, ahead/behind
  sonata lane land [--timeout 5m]              ask the user to merge your lane into its base
  sonata lane remove [--delete-branch]         ask the user to remove your lane

Orchestrator only:
  sonata recruit <claude|codex|opencode|gemini|shell|\"<command>\"> [--name N] [--role R | --role-text T] [--folder F | --lane]
  sonata lane status|land|remove <name>        the same for a linked terminal
  sonata dismiss <name>
  sonata note new \"<title>\" [\"<text>\"]       create a note linked to you
  sonata browser new [<url>]                  open a browser linked to you
  sonata link <a> <b>
  sonata unlink <a> <b>

Options: --timeout 10m (ask), --timeout 5m (lane land/remove), --json (any command).
Exit codes: 0 ok, 1 error, 2 usage, 3 not allowed, 4 not found, 5 timed out, 6 Sonata not reachable.";

/// Prints the help lines for one command (or everything for unknown ones).
fn print_help_for(command: &str) {
    let lines: Vec<&str> = HELP
        .lines()
        .filter(|l| l.trim_start().starts_with(&format!("sonata {command}")))
        .collect();
    if lines.is_empty() {
        println!("{HELP}");
    } else {
        for l in lines {
            println!("{}", l.trim_start());
        }
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("help" | "--help" | "-h") => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Some("run") => return run::main(args.split_off(1)),
        Some("_hook") => return hook::main(args.split_off(1)),
        _ => {}
    }
    // `--help` never runs a command: `sonata recruit --help` must not recruit anything.
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help_for(&args[0]);
        return ExitCode::SUCCESS;
    }
    let json = args.iter().any(|a| a == "--json");
    args.retain(|a| a != "--json");
    let stdin = if args.iter().any(|a| a == "--stdin") {
        args.retain(|a| a != "--stdin");
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_err() {
            eprintln!("sonata: could not read standard input");
            return ExitCode::from(EXIT_USAGE);
        }
        Some(buf)
    } else {
        None
    };
    client::run(args, stdin, json)
}
