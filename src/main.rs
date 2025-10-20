use clap::Parser;

use regex::Regex;

use std::error::Error;
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
struct Cli {
    path: String,

    /// Launch in designer mode
    #[arg(short, long)]
    designer: bool,
}

#[derive(Debug)]
enum PathKind {
    Server { host: String, ref_name: String },
    File { path: String },
    Web { url: String },
}

fn parse_base_path(input_path: &str) -> Result<PathKind, Box<dyn Error>> {
    let s = input_path.trim();

    if s.contains("ws=") {
        return parse_base_web_form(&s);
    }

    let s = s.to_lowercase();

    if s.contains("file=") {
        parse_base_file_form(&s)
    } else if s.contains("srvr=") && s.contains("ref=") {
        parse_base_server_form(&s)
    } else if s.contains(";") {
        parse_base_simple_form(&s)
    } else {
        Err(format!("Could not parse provided path: {input_path}").into())
    }
}

fn parse_base_simple_form(input: &str) -> Result<PathKind, Box<dyn Error>> {
    let captures = Regex::new(r"(.+);(.+)").unwrap().captures(input).ok_or("expected pattern: host;ref")?;
    Ok(PathKind::Server {
        host: captures[1].to_string(),
        ref_name: captures[2].to_string(),
    })
}

fn parse_base_server_form(input: &str) -> Result<PathKind, Box<dyn Error>> {
    let captures = Regex::new(r#""(.+)".+"(.+)""#).unwrap().captures(input).ok_or("expected pattern: Srvr=\"host\";Ref=\"ref\";")?;
    Ok(PathKind::Server {
        host: captures[1].to_string(),
        ref_name: captures[2].to_string(),
    })
}

fn parse_base_file_form(input: &str) -> Result<PathKind, Box<dyn Error>> {
    let captures = Regex::new(r#""(.+)""#).unwrap().captures(&input).ok_or("expected pattern: File=\"<path>\";")?;
    Ok(PathKind::File {
        path: captures[1].to_string(),
    })
}

fn parse_base_web_form(input: &str) -> Result<PathKind, Box<dyn Error>> {
    let captures = Regex::new(r#""(.+)""#).unwrap().captures(input).ok_or("expected pattern: ws=\"<url>\";")?;
    Ok(PathKind::Web {
        url: captures[1].to_string(),
    })
}

fn launch_base(path: PathKind, designer: bool) -> Result<(), Box<dyn Error>> {
    // TODO: add option to get 1cestart.exe path from cmd args or config file
    let starter = Path::new(r#"c:\Program Files\1cv8\common\1cestart.exe"#);

    if !starter.exists() {
        return Err(format!("Could not locate 1C starter app: '{}'", starter.display()).into())
    }

    let launch_mode = if designer { "DESIGNER" } else { "ENTERPRISE" };

    match path {
        PathKind::Server { host, ref_name } => {
            Command::new(starter)
                .args([launch_mode, "/S", &format!("{host}/{ref_name}")])
                .spawn()?;
        }

        PathKind::File { path } => {
            Command::new(starter)
                .args([launch_mode, "/F", &path])
                .spawn()?;
        }

        PathKind::Web { url } => {
            Command::new(starter)
                .args([launch_mode, "/WS", &url])
                .spawn()?;
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let parsed_path = match parse_base_path(&cli.path) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Parsing error: {}", e);
            std::process::exit(1);
        },
    };

    match launch_base(parsed_path, cli.designer) {
        Ok(()) => {},
        Err(e) => {
            eprintln!("Launcher error: {}", e);
            std::process::exit(1);
        }
    };

    Ok(())
}
