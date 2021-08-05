#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::let_underscore_drop)]
#![warn(clippy::cargo)]
#![allow(unknown_lints)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![warn(rust_2018_idioms)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(variant_size_differences)]

//! `artichoke` is the `ruby` binary frontend to Artichoke.
//!
//! `artichoke` supports executing programs via files, stdin, or inline with one
//! or more `-e` flags.
//!
//! Artichoke does not yet support reading from the local filesystem. A
//! temporary workaround is to inject data into the interpreter with the
//! `--with-fixture` flag, which reads file contents into a `$fixture` global.
//!
//! ```console
//! $ cargo run -q --bin artichoke -- --help
//! artichoke 0.1.0-pre.0
//! Artichoke is a Ruby made with Rust.
//!
//! USAGE:
//!     artichoke [FLAGS] [OPTIONS] [--] [programfile]...
//!
//! FLAGS:
//!         --copyright    print the copyright
//!     -h, --help         Prints help information
//!     -V, --version      Prints version information
//!
//! OPTIONS:
//!     -e <commands>...                one line of script. Several -e's allowed. Omit [programfile]
//!         --with-fixture <fixture>    file whose contents will be read into the `$fixture` global
//!
//! ARGS:
//!     <programfile>...
//! ```

use artichoke::ruby::{self, Args};
use clap::{App, AppSettings, Arg, ArgMatches};
use std::env;
use std::error;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use termcolor::{ColorChoice, StandardStream, WriteColor};

type Result<T> = ::std::result::Result<T, Box<dyn error::Error>>;

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(2);
        }
    };

    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    match ruby::run(args, io::stdin(), &mut stderr) {
        Ok(Ok(())) => {}
        Ok(Err(())) => process::exit(1),
        Err(err) => {
            // Reset colors and write the error message to stderr.
            //
            // Suppress all errors at this point (e.g. from a broken pipe) since
            // we're exiting with an error code anyway.
            let _ = stderr.reset();
            let _ = writeln!(stderr, "{}", err);
            process::exit(1);
        }
    }
}

fn parse_args() -> Result<Args> {
    let matches = clap_matches(env::args_os())?;

    let mut args = Args::empty()
        .with_copyright(matches.is_present("copyright"))
        .with_commands(
            matches
                .values_of_os("commands")
                .into_iter()
                .flat_map(|v| v.map(OsString::from))
                .collect(),
        )
        .with_fixture(matches.value_of_os("fixture").map(PathBuf::from));

    if let Some(mut positional) = matches.values_of_os("programfile") {
        if let Some(programfile) = positional.next() {
            args = args.with_programfile(Some(programfile.into()));
        }
        let ruby_program_argv = positional.map(OsString::from).collect::<Vec<_>>();
        args = args.with_argv(ruby_program_argv);
    }

    Ok(args)
}

fn app() -> App<'static, 'static> {
    let app = App::new("artichoke");
    let app = app.about("Artichoke is a Ruby made with Rust.");
    let app = app.arg(
        Arg::with_name("copyright")
            .takes_value(false)
            .multiple(false)
            .help("print the copyright")
            .long("copyright"),
    );
    let app = app.arg(
        Arg::with_name("commands")
            .takes_value(true)
            .multiple(true)
            .help(r"one line of script. Several -e's allowed. Omit [programfile]")
            .short("e"),
    );
    let app = app.arg(
        Arg::with_name("fixture")
            .takes_value(true)
            .multiple(false)
            .help("file whose contents will be read into the `$fixture` global")
            .long("with-fixture"),
    );
    let app = app.arg(Arg::with_name("programfile").takes_value(true).multiple(true));
    let app = app.version(env!("CARGO_PKG_VERSION"));
    app.setting(AppSettings::TrailingVarArg)
}

// NOTE: This routine is plucked from `ripgrep` as of
// 9f924ee187d4c62aa6ebe4903d0cfc6507a5adb5.
//
// `ripgrep` is licensed with the MIT License Copyright (c) 2015 Andrew Gallant.
//
// https://github.com/BurntSushi/ripgrep/blob/9f924ee187d4c62aa6ebe4903d0cfc6507a5adb5/LICENSE-MIT
//
// See https://github.com/artichoke/artichoke/issues/1301.

/// Returns a clap matches object if the given arguments parse successfully.
///
/// Otherwise, if an error occurred, then it is returned unless the error
/// corresponds to a `--help` or `--version` request. In which case, the
/// corresponding output is printed and the current process is exited
/// successfully.
fn clap_matches<I, T>(args: I) -> Result<ArgMatches<'static>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let err = match app().get_matches_from_safe(args) {
        Ok(matches) => return Ok(matches),
        Err(err) => err,
    };
    if err.use_stderr() {
        return Err(err.into());
    }
    // Explicitly ignore any error returned by write!. The most likely error
    // at this point is a broken pipe error, in which case, we want to ignore
    // it and exit quietly.
    //
    // (This is the point of this helper function. clap's functionality for
    // doing this will panic on a broken pipe error.)
    let _ = write!(io::stdout(), "{}", err);
    process::exit(0);
}
