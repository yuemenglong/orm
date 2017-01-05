// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(not(test))]

// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// TODO we're going to allocate a whole bunch of temp Strings, is it worth
// keeping some scratch mem for this and running our own StrPool?
// TODO for lint violations of names, emit a refactor script

#[macro_use]
extern crate log;

extern crate syntex_syntax as syntax;
extern crate syntex_errors as errors;
extern crate rustc_serialize;

extern crate strings;

extern crate unicode_segmentation;
extern crate regex;
extern crate diff;
extern crate term;
extern crate itertools;
extern crate multimap;

use errors::{Handler, DiagnosticBuilder};
use errors::emitter::{ColorConfig, EmitterWriter};
use syntax::ast;
use syntax::codemap::{mk_sp, CodeMap, Span};
use syntax::parse::{self, ParseSess};

use strings::string_buffer::StringBuffer;

use std::io::{self, stdout, Write};
use std::ops::{Add, Sub};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::collections::HashMap;
use std::fmt;

use issues::{BadIssueSeeker, Issue};
use filemap::FileMap;
use visitor::FmtVisitor;
use config::Config;
use checkstyle::{output_header, output_footer};

pub use self::summary::Summary;

#[macro_use]
mod utils;
pub mod config;
pub mod codemap;
pub mod filemap;
pub mod file_lines;
pub mod visitor;
mod checkstyle;
mod items;
mod missed_spans;
mod lists;
mod types;
mod expr;
mod imports;
mod issues;
mod rewrite;
mod string;
mod comment;
pub mod modules;
pub mod rustfmt_diff;
mod chains;
mod macros;
mod patterns;
mod summary;

const MIN_STRING: usize = 10;
// When we get scoped annotations, we should have rustfmt::skip.
const SKIP_ANNOTATION: &'static str = "rustfmt_skip";

pub trait Spanned {
    fn span(&self) -> Span;
}

impl Spanned for ast::Expr {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ast::Pat {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ast::Ty {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ast::Arg {
    fn span(&self) -> Span {
        if items::is_named_arg(self) {
            mk_sp(self.pat.span.lo, self.ty.span.hi)
        } else {
            self.ty.span
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Indent {
    // Width of the block indent, in characters. Must be a multiple of
    // Config::tab_spaces.
    pub block_indent: usize,
    // Alignment in characters.
    pub alignment: usize,
}

impl Indent {
    pub fn new(block_indent: usize, alignment: usize) -> Indent {
        Indent {
            block_indent: block_indent,
            alignment: alignment,
        }
    }

    pub fn empty() -> Indent {
        Indent::new(0, 0)
    }

    pub fn block_indent(mut self, config: &Config) -> Indent {
        self.block_indent += config.tab_spaces;
        self
    }

    pub fn block_unindent(mut self, config: &Config) -> Indent {
        self.block_indent -= config.tab_spaces;
        self
    }

    pub fn width(&self) -> usize {
        self.block_indent + self.alignment
    }

    pub fn to_string(&self, config: &Config) -> String {
        let (num_tabs, num_spaces) = if config.hard_tabs {
            (self.block_indent / config.tab_spaces, self.alignment)
        } else {
            (0, self.block_indent + self.alignment)
        };
        let num_chars = num_tabs + num_spaces;
        let mut indent = String::with_capacity(num_chars);
        for _ in 0..num_tabs {
            indent.push('\t')
        }
        for _ in 0..num_spaces {
            indent.push(' ')
        }
        indent
    }
}

impl Add for Indent {
    type Output = Indent;

    fn add(self, rhs: Indent) -> Indent {
        Indent {
            block_indent: self.block_indent + rhs.block_indent,
            alignment: self.alignment + rhs.alignment,
        }
    }
}

impl Sub for Indent {
    type Output = Indent;

    fn sub(self, rhs: Indent) -> Indent {
        Indent::new(self.block_indent - rhs.block_indent,
                    self.alignment - rhs.alignment)
    }
}

impl Add<usize> for Indent {
    type Output = Indent;

    fn add(self, rhs: usize) -> Indent {
        Indent::new(self.block_indent, self.alignment + rhs)
    }
}

impl Sub<usize> for Indent {
    type Output = Indent;

    fn sub(self, rhs: usize) -> Indent {
        Indent::new(self.block_indent, self.alignment - rhs)
    }
}

pub enum ErrorKind {
    // Line has exceeded character limit
    LineOverflow,
    // Line ends in whitespace
    TrailingWhitespace,
    // TO-DO or FIX-ME item without an issue number
    BadIssue(Issue),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            ErrorKind::LineOverflow => write!(fmt, "line exceeded maximum length"),
            ErrorKind::TrailingWhitespace => write!(fmt, "left behind trailing whitespace"),
            ErrorKind::BadIssue(issue) => write!(fmt, "found {}", issue),
        }
    }
}

// Formatting errors that are identified *after* rustfmt has run.
pub struct FormattingError {
    line: u32,
    kind: ErrorKind,
}

impl FormattingError {
    fn msg_prefix(&self) -> &str {
        match self.kind {
            ErrorKind::LineOverflow |
            ErrorKind::TrailingWhitespace => "Rustfmt failed at",
            ErrorKind::BadIssue(_) => "WARNING:",
        }
    }

    fn msg_suffix(&self) -> &str {
        match self.kind {
            ErrorKind::LineOverflow |
            ErrorKind::TrailingWhitespace => "(sorry)",
            ErrorKind::BadIssue(_) => "",
        }
    }
}

pub struct FormatReport {
    // Maps stringified file paths to their associated formatting errors.
    file_error_map: HashMap<String, Vec<FormattingError>>,
}

impl FormatReport {
    fn new() -> FormatReport {
        FormatReport { file_error_map: HashMap::new() }
    }

    pub fn warning_count(&self) -> usize {
        self.file_error_map.iter().map(|(_, errors)| errors.len()).fold(0, |acc, x| acc + x)
    }

    pub fn has_warnings(&self) -> bool {
        self.warning_count() > 0
    }
}

impl fmt::Display for FormatReport {
    // Prints all the formatting errors.
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for (file, errors) in &self.file_error_map {
            for error in errors {
                try!(write!(fmt,
                            "{} {}:{}: {} {}\n",
                            error.msg_prefix(),
                            file,
                            error.line,
                            error.kind,
                            error.msg_suffix()));
            }
        }
        Ok(())
    }
}

// Formatting which depends on the AST.
fn format_ast<F>(krate: &ast::Crate,
                 parse_session: &ParseSess,
                 main_file: &Path,
                 config: &Config,
                 mut after_file: F)
                 -> Result<(FileMap, bool), io::Error>
    where F: FnMut(&str, &mut StringBuffer) -> Result<bool, io::Error>
{
    let mut result = FileMap::new();
    // diff mode: check if any files are differing
    let mut has_diff = false;

    // We always skip children for the "Plain" write mode, since there is
    // nothing to distinguish the nested module contents.
    let skip_children = config.skip_children || config.write_mode == config::WriteMode::Plain;
    for (path, module) in modules::list_files(krate, parse_session.codemap()) {
        if skip_children && path.as_path() != main_file {
            continue;
        }
        let path = path.to_str().unwrap();
        if config.verbose {
            println!("Formatting {}", path);
        }
        let mut visitor = FmtVisitor::from_codemap(parse_session, config);
        visitor.format_separate_mod(module);

        has_diff |= try!(after_file(path, &mut visitor.buffer));

        result.push((path.to_owned(), visitor.buffer));
    }

    Ok((result, has_diff))
}

// Formatting done on a char by char or line by line basis.
// FIXME(#209) warn on bad license
// FIXME(#20) other stuff for parity with make tidy
fn format_lines(text: &mut StringBuffer, name: &str, config: &Config, report: &mut FormatReport) {
    // Iterate over the chars in the file map.
    let mut trims = vec![];
    let mut last_wspace: Option<usize> = None;
    let mut line_len = 0;
    let mut cur_line = 1;
    let mut newline_count = 0;
    let mut errors = vec![];
    let mut issue_seeker = BadIssueSeeker::new(config.report_todo, config.report_fixme);

    for (c, b) in text.chars() {
        if c == '\r' {
            line_len += c.len_utf8();
            continue;
        }

        // Add warnings for bad todos/ fixmes
        if let Some(issue) = issue_seeker.inspect(c) {
            errors.push(FormattingError {
                line: cur_line,
                kind: ErrorKind::BadIssue(issue),
            });
        }

        if c == '\n' {
            // Check for (and record) trailing whitespace.
            if let Some(lw) = last_wspace {
                trims.push((cur_line, lw, b));
                line_len -= b - lw;
            }
            // Check for any line width errors we couldn't correct.
            if line_len > config.max_width {
                errors.push(FormattingError {
                    line: cur_line,
                    kind: ErrorKind::LineOverflow,
                });
            }
            line_len = 0;
            cur_line += 1;
            newline_count += 1;
            last_wspace = None;
        } else {
            newline_count = 0;
            line_len += c.len_utf8();
            if c.is_whitespace() {
                if last_wspace.is_none() {
                    last_wspace = Some(b);
                }
            } else {
                last_wspace = None;
            }
        }
    }

    if newline_count > 1 {
        debug!("track truncate: {} {}", text.len, newline_count);
        let line = text.len - newline_count + 1;
        text.truncate(line);
    }

    for &(l, _, _) in &trims {
        errors.push(FormattingError {
            line: l,
            kind: ErrorKind::TrailingWhitespace,
        });
    }

    report.file_error_map.insert(name.to_owned(), errors);
}

fn parse_input(input: Input,
               parse_session: &ParseSess)
               -> Result<ast::Crate, Option<DiagnosticBuilder>> {
    let result = match input {
        Input::File(file) => parse::parse_crate_from_file(&file, parse_session),
        Input::Text(text) => {
            parse::parse_crate_from_source_str("stdin".to_owned(), text, parse_session)
        }
    };

    match result {
        Ok(c) => {
            if parse_session.span_diagnostic.has_errors() {
                // Bail out if the parser recovered from an error.
                Err(None)
            } else {
                Ok(c)
            }
        }
        Err(e) => Err(Some(e)),
    }
}

fn create_parse_session() ->ParseSess{
    let codemap = Rc::new(CodeMap::new());
    let tty_handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());
    parse_session
}

pub fn format_input<T: Write>(input: Input,
                              config: &Config,
                              mut out: Option<&mut T>)
                              -> Result<(Summary, FileMap, FormatReport), (io::Error, Summary)> {
    let mut summary = Summary::new();
    let codemap = Rc::new(CodeMap::new());

    let tty_handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let mut parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());

    let main_file = match input {
        Input::File(ref file) => file.clone(),
        Input::Text(..) => PathBuf::from("stdin"),
    };

    let krate = match parse_input(input, &parse_session) {
        Ok(krate) => krate,
        Err(diagnostic) => {
            if let Some(mut diagnostic) = diagnostic {
                diagnostic.emit();
            }
            summary.add_parsing_error();
            return Ok((summary, FileMap::new(), FormatReport::new()));
        }
    };

    if parse_session.span_diagnostic.has_errors() {
        summary.add_parsing_error();
    }

    // Suppress error output after parsing.
    let silent_emitter = Box::new(EmitterWriter::new(Box::new(Vec::new()), Some(codemap.clone())));
    parse_session.span_diagnostic = Handler::with_emitter(true, false, silent_emitter);

    let mut report = FormatReport::new();

    match format_ast(&krate,
                     &parse_session,
                     &main_file,
                     config,
                     |file_name, file| {
        // For some reason, the codemap does not include terminating
        // newlines so we must add one on for each file. This is sad.
        filemap::append_newline(file);

        format_lines(file, file_name, config, &mut report);

        if let Some(ref mut out) = out {
            return filemap::write_file(file, file_name, out, config);
        }
        Ok(false)
    }) {
        Ok((file_map, has_diff)) => {
            if report.has_warnings() {
                summary.add_formatting_error();
            }

            if has_diff {
                summary.add_diff();
            }

            Ok((summary, file_map, report))
        }
        Err(e) => Err((e, summary)),
    }
}

#[derive(Debug)]
pub enum Input {
    File(PathBuf),
    Text(String),
}

pub fn run(input: Input, config: &Config) -> Summary {
    let mut out = &mut stdout();
    output_header(out, config.write_mode).ok();
    match format_input(input, config, Some(out)) {
        Ok((summary, _, report)) => {
            output_footer(out, config.write_mode).ok();

            if report.has_warnings() {
                msg!("{}", report);
            }

            summary
        }
        Err((msg, mut summary)) => {
            msg!("Error writing files: {}", msg);
            summary.add_operational_error();
            summary
        }
    }
}




// extern crate log;
// extern crate rustfmt;
extern crate toml;
extern crate env_logger;
extern crate getopts;

// use rustfmt::{run, Input, Summary};
// use rustfmt::file_lines::FileLines;
// use rustfmt::config::{Config, WriteMode};

use file_lines::FileLines;
// use config::{Config, WriteMode};
use config::WriteMode;

use std::{env, error};
use std::fs::{self, File};
use std::io::{Read};
// use std::path::{PathBuf};
use std::str::FromStr;

use getopts::{Matches, Options};

// Include git commit hash and worktree status; contents are like
//   const COMMIT_HASH: Option<&'static str> = Some("c31a366");
//   const WORKTREE_CLEAN: Option<bool> = Some(false);
// with `None` if running git failed, eg if it is not installed.
// include!(concat!(env!("OUT_DIR"), "/git_info.rs"));

type FmtError = Box<error::Error + Send + Sync>;
type FmtResult<T> = std::result::Result<T, FmtError>;

/// Rustfmt operations.
enum Operation {
    /// Format files and their child modules.
    Format {
        files: Vec<PathBuf>,
        config_path: Option<PathBuf>,
    },
    /// Print the help message.
    Help,
    // Print version information
    Version,
    /// Print detailed configuration help.
    ConfigHelp,
    /// No file specified, read from stdin
    Stdin {
        input: String,
        config_path: Option<PathBuf>,
    },
}

/// Parsed command line options.
#[derive(Clone, Debug, Default)]
struct CliOptions {
    skip_children: bool,
    verbose: bool,
    write_mode: Option<WriteMode>,
    file_lines: FileLines, // Default is all lines in all files.
}

impl CliOptions {
    fn from_matches(matches: &Matches) -> FmtResult<CliOptions> {
        let mut options = CliOptions::default();
        options.skip_children = matches.opt_present("skip-children");
        options.verbose = matches.opt_present("verbose");

        if let Some(ref write_mode) = matches.opt_str("write-mode") {
            if let Ok(write_mode) = WriteMode::from_str(write_mode) {
                options.write_mode = Some(write_mode);
            } else {
                return Err(FmtError::from(format!("Invalid write-mode: {}", write_mode)));
            }
        }

        if let Some(ref file_lines) = matches.opt_str("file-lines") {
            options.file_lines = try!(file_lines.parse());
        }

        Ok(options)
    }

    fn apply_to(self, config: &mut Config) {
        config.skip_children = self.skip_children;
        config.verbose = self.verbose;
        config.file_lines = self.file_lines;
        if let Some(write_mode) = self.write_mode {
            config.write_mode = write_mode;
        }
    }
}

/// Try to find a project file in the given directory and its parents. Returns the path of a the
/// nearest project file if one exists, or `None` if no project file was found.
fn lookup_project_file(dir: &Path) -> FmtResult<Option<PathBuf>> {
    let mut current = if dir.is_relative() {
        try!(env::current_dir()).join(dir)
    } else {
        dir.to_path_buf()
    };

    current = try!(fs::canonicalize(current));

    const CONFIG_FILE_NAMES: [&'static str; 2] = [".rustfmt.toml", "rustfmt.toml"];

    loop {
        for config_file_name in &CONFIG_FILE_NAMES {
            let config_file = current.join(config_file_name);
            match fs::metadata(&config_file) {
                // Only return if it's a file to handle the unlikely situation of a directory named
                // `rustfmt.toml`.
                Ok(ref md) if md.is_file() => return Ok(Some(config_file)),
                // Return the error if it's something other than `NotFound`; otherwise we didn't
                // find the project file yet, and continue searching.
                Err(e) => {
                    // if e.kind() != ErrorKind::NotFound {
                        return Err(FmtError::from(e));
                    // }
                }
                _ => {}
            }
        }

        // If the current directory has no parent, we're done searching.
        if !current.pop() {
            return Ok(None);
        }
    }
}

/// Resolve the config for input in `dir`.
///
/// Returns the `Config` to use, and the path of the project file if there was
/// one.
fn resolve_config(dir: &Path) -> FmtResult<(Config, Option<PathBuf>)> {
    let path = try!(lookup_project_file(dir));
    if path.is_none() {
        return Ok((Config::default(), None));
    }
    let path = path.unwrap();
    let mut file = try!(File::open(&path));
    let mut toml = String::new();
    try!(file.read_to_string(&mut toml));
    Ok((Config::from_toml(&toml), Some(path)))
}

/// read the given config file path recursively if present else read the project file path
fn match_cli_path_or_file(config_path: Option<PathBuf>,
                          input_file: &Path)
                          -> FmtResult<(Config, Option<PathBuf>)> {

    if let Some(config_file) = config_path {
        let (toml, path) = try!(resolve_config(config_file.as_ref()));
        if path.is_some() {
            return Ok((toml, path));
        }
    }
    resolve_config(input_file)
}

fn make_opts() -> Options {
    let mut opts = Options::new();
    opts.optflag("h", "help", "show this message");
    opts.optflag("V", "version", "show version information");
    opts.optflag("v", "verbose", "show progress");
    opts.optopt("",
                "write-mode",
                "mode to write in (not usable when piping from stdin)",
                "[replace|overwrite|display|diff|coverage|checkstyle]");
    opts.optflag("", "skip-children", "don't reformat child modules");

    opts.optflag("",
                 "config-help",
                 "show details of rustfmt configuration options");
    opts.optopt("",
                "config-path",
                "Recursively searches the given path for the rustfmt.toml config file. If not \
                 found reverts to the input file path",
                "[Path for the configuration file]");
    opts.optopt("",
                "file-lines",
                "Format specified line ranges. See README for more detail on the JSON format.",
                "JSON");

    opts
}

fn execute(opts: &Options) -> FmtResult<Summary> {
    let matches = try!(opts.parse(env::args().skip(1)));

    match try!(determine_operation(&matches)) {
        Operation::Help => {
            print_usage(opts, "");
            Ok(Summary::new())
        }
        Operation::Version => {
            print_version();
            Ok(Summary::new())
        }
        Operation::ConfigHelp => {
            Config::print_docs();
            Ok(Summary::new())
        }
        Operation::Stdin { input, config_path } => {
            // try to read config from local directory
            let (mut config, _) = match_cli_path_or_file(config_path, &env::current_dir().unwrap())
                .expect("Error resolving config");

            // write_mode is always Plain for Stdin.
            config.write_mode = WriteMode::Plain;

            Ok(run(Input::Text(input), &config))
        }
        Operation::Format { mut files, config_path } => {
            let options = try!(CliOptions::from_matches(&matches));

            // Add any additional files that were specified via `--file-lines`.
            files.extend(options.file_lines.files().cloned().map(PathBuf::from));

            let mut config = Config::default();
            let mut path = None;
            // Load the config path file if provided
            if let Some(config_file) = config_path {
                let (cfg_tmp, path_tmp) = resolve_config(config_file.as_ref())
                    .expect(&format!("Error resolving config for {:?}", config_file));
                config = cfg_tmp;
                path = path_tmp;
            };
            if let Some(path) = path.as_ref() {
                println!("Using rustfmt config file {}", path.display());
            }

            let mut error_summary = Summary::new();
            for file in files {
                // Check the file directory if the config-path could not be read or not provided
                // if path.is_none() {
                //     let (config_tmp, path_tmp) = resolve_config(file.parent().unwrap())
                //         .expect(&format!("Error resolving config for {}", file.display()));
                //     if let Some(path) = path_tmp.as_ref() {
                //         println!("Using rustfmt config file {} for {}",
                //                  path.display(),
                //                  file.display());
                //     }
                //     config = config_tmp;
                // }

                options.clone().apply_to(&mut config);
                error_summary.add(run(Input::File(file), &config));
            }
            Ok(error_summary)
        }
    }
}

fn main() {
    let mut parse_session = create_parse_session();
    let krate = parse::parse_crate_from_source_str("stdin".to_string(), "fn main(){}".to_string(), &parse_session).unwrap();
    println!("{:?}", krate.module.items);
    println!("{:?}", krate.module.items.len());
    std::process::exit(0);

    let _ = env_logger::init();

    let opts = make_opts();

    let exit_code = match execute(&opts) {
        Ok(summary) => {
            if summary.has_operational_errors() {
                1
            } else if summary.has_parsing_errors() {
                2
            } else if summary.has_formatting_errors() {
                3
            } else if summary.has_diff {
                // should only happen in diff mode
                4
            } else {
                assert!(summary.has_no_errors());
                0
            }
        }
        Err(e) => {
            print_usage(&opts, &e.to_string());
            1
        }
    };
    // Make sure standard output is flushed before we exit.
    std::io::stdout().flush().unwrap();

    // Exit with given exit code.
    //
    // NOTE: This immediately terminates the process without doing any cleanup,
    // so make sure to finish all necessary cleanup before this is called.
    std::process::exit(exit_code);
}

fn print_usage(opts: &Options, reason: &str) {
    let reason = format!("{}\nusage: {} [options] <file>...",
                         reason,
                         env::args_os().next().unwrap().to_string_lossy());
    println!("{}", opts.usage(&reason));
}

fn print_version() {
    // println!("{} ({}{})",
    //          option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"),
    //          COMMIT_HASH.unwrap_or("git commit unavailable"),
    //          match WORKTREE_CLEAN {
    //              Some(false) => " worktree dirty",
    //              _ => "",
    //          });
}

fn determine_operation(matches: &Matches) -> FmtResult<Operation> {
    if matches.opt_present("h") {
        return Ok(Operation::Help);
    }

    if matches.opt_present("config-help") {
        return Ok(Operation::ConfigHelp);
    }

    if matches.opt_present("version") {
        return Ok(Operation::Version);
    }

    // Read the config_path and convert to parent dir if a file is provided.
    let config_path: Option<PathBuf> = matches.opt_str("config-path")
        .map(PathBuf::from)
        .and_then(|dir| {
            if dir.is_file() {
                return dir.parent().map(|v| v.into());
            }
            Some(dir)
        });

    // if no file argument is supplied and `--file-lines` is not specified, read from stdin
    if matches.free.is_empty() && !matches.opt_present("file-lines") {

        let mut buffer = String::new();
        try!(io::stdin().read_to_string(&mut buffer));

        return Ok(Operation::Stdin {
            input: buffer,
            config_path: config_path,
        });
    }

    // We append files from `--file-lines` later in `execute()`.
    let files: Vec<_> = matches.free.iter().map(PathBuf::from).collect();

    Ok(Operation::Format {
        files: files,
        config_path: config_path,
    })
}
