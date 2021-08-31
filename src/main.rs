// use colored::*;
use std::str::FromStr;

use anyhow::anyhow;
use clap::AppSettings;
use clap::Clap;
use colored::Colorize;
// use reqwest::Error;
use mime::Mime;
use reqwest::{header, Url};

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

// 3 slash means help message for cli
/// httpie in rust rather than in python
#[derive(Clap, Debug)]
#[clap(name = "httpie-rs")]
#[clap(version = "0.1.0", author = "jay")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    /// print debug info
    #[clap(short, long)]
    debug: bool,
    #[clap(subcommand)]
    sub_cmd: SubCmd,
}

#[derive(Clap, Debug)]
enum SubCmd {
    Get(Get),
    Post(Post),
}

// 3 slash means help message for cli
/// fire a http get request for  you
#[derive(Clap, Debug)]
struct Get {
    #[clap(parse(try_from_str = parse_url))]
    url: String,
}

#[derive(Debug)]
struct UrlKV(String, String);

impl FromStr for UrlKV {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let kv = s.split('=').filter(|c| !(c.is_empty())).collect::<Vec<_>>();
        if kv.len() != 2 {
            return Err(anyhow!(format!("parse url param error:{}", s)));
        }
        Ok(Self(kv[0].into(), kv[1].into()))
    }
}

// 3 slash means help message for cli
/// fire a http post request for  you
#[derive(Clap, Debug)]
struct Post {
    #[clap(parse(try_from_str = parse_url))]
    url: String,
    #[clap(parse(try_from_str = parse_url_param))]
    body: Vec<UrlKV>,
}

fn parse_url(s: &str) -> anyhow::Result<String> {
    let _ = s.parse::<Url>()?;
    Ok(s.into())
}

fn parse_url_param(s: &str) -> anyhow::Result<UrlKV> {
    s.parse::<UrlKV>()
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();
    // println!("opts: {:?}", opts);
    let mut header = header::HeaderMap::new();
    header.insert(header::USER_AGENT, "httpie-rs".parse()?);
    let client = reqwest::Client::builder().default_headers(header).build()?;

    let rsp;
    match opts.sub_cmd {
        SubCmd::Get(Get { ref url }) => {
            rsp = client.get(url).send().await?;
        }
        SubCmd::Post(Post { ref url, ref body }) => {
            let mut json_body = std::collections::HashMap::new();
            for v in body.iter() {
                json_body.insert(&v.0, &v.1);
            }
            rsp = client.post(url).json(&json_body).send().await?;
        }
    }

    print_response(rsp).await;
    Ok(())
}

async fn print_response(rsp: reqwest::Response) {
    // status
    println!("{:?} {}\n", rsp.version(), rsp.status().to_string().red());
    // headers
    rsp.headers().iter().for_each(|h| {
        println!("{}: {:?}", h.0.to_string().green(), h.1);
    });
    // body
    // get content-type
    if let Some(m) = rsp
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|h| h.to_str().unwrap().parse::<Mime>().unwrap())
    {
        match m {
            v if v == mime::APPLICATION_JSON => {
                // println!(
                // "{}",
                // jsonxf::pretty_print(&rsp.text().await.unwrap())
                // .unwrap()
                // .bright_purple()
                // );
                let ps = SyntaxSet::load_defaults_newlines();
                let ts = ThemeSet::load_defaults();
                let syntax = ps.find_syntax_by_extension("rs").unwrap();
                let mut h = HighlightLines::new(syntax, &ts.themes["base16-eighties.dark"]);
                for line in LinesWithEndings::from(&rsp.text().await.unwrap()) {
                    let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
                    println!("{}", escaped);
                }
                println!("\x1b[0m");
            }
            _ => {
                println!("{:?}", &rsp.text().await.unwrap());
            }
        }
    } else {
        println!("{}", rsp.text().await.unwrap())
    }
}
