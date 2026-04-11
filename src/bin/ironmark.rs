//! `ironmark` CLI — render Markdown files as coloured terminal output.
//!
//! # Usage
//!
//! ```text
//! ironmark --ansi [OPTIONS] [FILE...]
//! ```
//!
//! When no `FILE` arguments are given, input is read from stdin.
//! Use `-` as a file path to explicitly read from stdin.
//!
//! # Options
//!
//! | Flag | Description |
//! |---|---|
//! | `--ansi` | Render as ANSI-coloured terminal output (required) |
//! | `--width N` | Terminal column width for word-wrap and heading underlines (default: auto-detect via `$COLUMNS` / `tput cols`, fallback 80) |
//! | `--no-color` / `--no-colour` | Disable ANSI escape codes (plain text output) |
//! | `-n`, `--line-numbers` | Show line numbers in fenced code blocks |
//! | `--no-hard-breaks` | Don't convert newlines inside paragraphs to `<br>` |
//! | `--no-tables` | Disable pipe table syntax |
//! | `--no-highlight` | Disable `==highlight==` syntax |
//! | `--no-strikethrough` | Disable `~~strikethrough~~` syntax |
//! | `--no-underline` | Disable `++underline++` syntax |
//! | `--no-autolink` | Disable bare URL auto-linking |
//! | `--no-task-lists` | Disable `- [x]` task list syntax |
//! | `--math` | Enable `$inline$` and `$$display$$` math syntax |
//! | `--wiki-links` | Enable `[[wiki link]]` syntax |
//! | `--max-size N` | Truncate input to N bytes (0 = unlimited, default: 0) |
//! | `--help`, `-h` | Print this help and exit |
//! | `--version`, `-V` | Print version and exit |
//!
//! # Examples
//!
//! ```text
//! ironmark --ansi README.md
//! ironmark --ansi --no-color README.md | less
//! ironmark --ansi --width 80 README.md
//! echo "# Hello **world**" | ironmark --ansi
//! ```

use std::io::{self, Read};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
ironmark — render Markdown as coloured terminal output

USAGE:
    ironmark --ansi [OPTIONS] [FILE...]

    When no FILE is given, reads from stdin. Use '-' for stdin explicitly.

OPTIONS:
    --ansi               Render as ANSI-coloured terminal output (required)
    --width N            Terminal column width for word-wrap and heading underlines
                         (default: auto-detect via $COLUMNS / tput cols, fallback 80)
    --no-color           Disable ANSI escape codes (plain text)
    -n, --line-numbers   Show line numbers in fenced code blocks
    --no-hard-breaks     Don't turn soft newlines into hard line breaks
    --no-tables          Disable pipe table syntax
    --no-highlight       Disable ==highlight== syntax
    --no-strikethrough   Disable ~~strikethrough~~ syntax
    --no-underline       Disable ++underline++ syntax
    --no-autolink        Disable bare URL auto-linking
    --no-task-lists      Disable - [x] task list syntax
    --math               Enable $inline$ and $$display$$ math
    --wiki-links         Enable [[wiki link]] syntax
    --padding N          Add horizontal padding to ANSI output
    --max-size N         Truncate input to N bytes (0 = unlimited)
    -h, --help           Print this help and exit
    -V, --version        Print version and exit

EXAMPLES:
    ironmark --ansi README.md
    ironmark --ansi --width 120 README.md
    ironmark --ansi --no-color README.md | less
    echo '# Hello' | ironmark --ansi
    cat doc.md | ironmark --ansi --math --wiki-links
";

fn parse_usize_arg(flag: &str, value: &str) -> usize {
    value.parse::<usize>().unwrap_or_else(|_| {
        eprintln!("error: {flag} value must be a non-negative integer");
        std::process::exit(2);
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut files: Vec<String> = Vec::new();
    let mut ansi_mode = false;
    let mut color = true;
    let mut line_numbers = false;
    let mut width: Option<usize> = None;
    let mut padding: usize = 0;
    let mut parse_opts = ironmark::ParseOptions::default();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return;
            }
            "-V" | "--version" => {
                println!("ironmark {VERSION}");
                return;
            }
            "--ansi" => ansi_mode = true,
            "--no-color" | "--no-colour" => color = false,
            "--line-numbers" | "-n" => line_numbers = true,
            "--no-hard-breaks" => parse_opts.hard_breaks = false,
            "--no-tables" => parse_opts.enable_tables = false,
            "--no-highlight" => parse_opts.enable_highlight = false,
            "--no-strikethrough" => parse_opts.enable_strikethrough = false,
            "--no-underline" => parse_opts.enable_underline = false,
            "--no-autolink" => parse_opts.enable_autolink = false,
            "--no-task-lists" => parse_opts.enable_task_lists = false,
            "--math" => parse_opts.enable_latex_math = true,
            "--wiki-links" => parse_opts.enable_wiki_links = true,
            "--width" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --width requires a value");
                    std::process::exit(2);
                }
                width = Some(parse_usize_arg("--width", &args[i]));
            }
            "--padding" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --padding requires a value");
                    std::process::exit(2);
                }
                padding = parse_usize_arg("--padding", &args[i]);
            }
            "--max-size" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --max-size requires a value");
                    std::process::exit(2);
                }
                parse_opts.max_input_size = parse_usize_arg("--max-size", &args[i]);
            }
            arg if arg.starts_with("--width=") => {
                width = Some(parse_usize_arg("--width", &arg["--width=".len()..]));
            }
            arg if arg.starts_with("--padding=") => {
                padding = parse_usize_arg("--padding", &arg["--padding=".len()..]);
            }
            arg if arg.starts_with("--max-size=") => {
                parse_opts.max_input_size =
                    parse_usize_arg("--max-size", &arg["--max-size=".len()..]);
            }
            arg if arg.starts_with('-') && arg != "-" => {
                eprintln!("error: unknown flag: {arg}");
                eprintln!("Run 'ironmark --help' for usage.");
                std::process::exit(2);
            }
            path => files.push(path.to_string()),
        }
        i += 1;
    }

    if !ansi_mode {
        eprintln!("error: --ansi flag is required");
        eprintln!("Run 'ironmark --help' for usage.");
        std::process::exit(2);
    }

    // Resolve terminal width: explicit --width → $COLUMNS env var → tput cols → fallback 80
    let resolved_width = width.unwrap_or_else(|| {
        // $COLUMNS is set by most shells when running interactively
        if let Some(w) = std::env::var("COLUMNS")
            .ok()
            .and_then(|v| v.trim().parse().ok())
        {
            return w;
        }
        // Last resort: ask tput (only works when a TTY is attached)
        if let Ok(out) = std::process::Command::new("tput").arg("cols").output()
            && out.status.success()
            && let Ok(s) = std::str::from_utf8(&out.stdout)
            && let Ok(n) = s.trim().parse::<usize>()
            && n > 0
        {
            return n;
        }
        80
    });

    let aopts = ironmark::AnsiOptions {
        width: resolved_width,
        color,
        line_numbers,
        padding,
    };

    let mut input = String::new();

    if files.is_empty() {
        io::stdin().read_to_string(&mut input).unwrap_or_else(|e| {
            eprintln!("error: failed to read stdin: {e}");
            std::process::exit(1);
        });
    } else {
        for path in &files {
            if path == "-" {
                io::stdin().read_to_string(&mut input).unwrap_or_else(|e| {
                    eprintln!("error: failed to read stdin: {e}");
                    std::process::exit(1);
                });
            } else {
                match std::fs::read_to_string(path) {
                    Ok(content) => input.push_str(&content),
                    Err(e) => {
                        eprintln!("error: {path}: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    print!(
        "{}",
        ironmark::render_ansi(&input, &parse_opts, Some(&aopts))
    );
}
