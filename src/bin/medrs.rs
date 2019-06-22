extern crate config;
extern crate mediawiki;
extern crate wikibase;

use docopt::Docopt;
use mediawiki::api::Api;
use serde::Deserialize;
use std::str;
use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

fn lines_from_file(filename: &str) -> Vec<String> {
    if filename.is_empty() {
        return vec![];
    }
    let file = File::open(filename).expect(format!("no such file: {}", filename).as_str());
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

fn read_file_to_string(filename: &str) -> String {
    let mut file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => panic!("no such file"),
    };
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)
        .ok()
        .expect("failed to read!");
    file_contents
}

fn replace_sparql_placeolder(pattern: &str, sparql: &String, lines: &Vec<String>) -> String {
    let rep: String = if lines.is_empty() {
        "".to_string()
    } else {
        "wd:".to_string() + &lines.join(" wd:")
    };
    sparql.replace(pattern, &rep)
}

fn output_sparql_result_items(sparql: &String) {
    let api = Api::new("https://www.wikidata.org/w/api.php").expect("Can't connect to Wikidata");
    let result = api.sparql_query(&sparql).expect("SPARQL query failed");
    let varname = result["head"]["vars"][0]
        .as_str()
        .expect("Can't find first variable name in SPARQL result");
    let entities = api.entities_from_sparql_result(&result, &varname);
    println!("{}", entities.join("\n"));
}

/*
fn get_all_from_stdin() -> String {
    let mut payload = Vec::new();
    io::stdin().read_to_end(&mut payload).unwrap();
    let s = match str::from_utf8(&payload) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };
    s.to_string()
}
*/

fn command_query(args: &Args) {
    if args.arg_query.is_empty() {
        println!("Requires SPARQL query");
    }

    let sparql = &args.arg_query;
    output_sparql_result_items(&sparql);
}

fn command_run(args: &Args) {
    let articles = lines_from_file(&args.flag_articles);
    let reviews = lines_from_file(&args.flag_reviews);
    let topics = lines_from_file(&args.flag_topics);
    let journals = lines_from_file(&args.flag_journals);
    let publishers = lines_from_file(&args.flag_publishers);
    let mut sparql = read_file_to_string(&args.flag_sparql);

    sparql = replace_sparql_placeolder("%%ARTICLES%%", &sparql, &articles);
    sparql = replace_sparql_placeolder("%%REVIEWS%%", &sparql, &reviews);
    sparql = replace_sparql_placeolder("%%TOPICS%%", &sparql, &topics);
    sparql = replace_sparql_placeolder("%%JOURNALS%%", &sparql, &journals);
    sparql = replace_sparql_placeolder("%%PUBLISHERS%%", &sparql, &publishers);

    output_sparql_result_items(&sparql);
}

const USAGE: &'static str = "
MEDRS

Usage:
    medrs run [--articles=<file>] [--reviews=<file>] [--topics=<file>] [--journals=<file>] [--publishers=<file>] [--sparql=<file>]
    medrs query <query>
    medrs (-h | --help)
    medrs --version

Options:
    -h --help            Show this screen.
    --version            Show version.
    --reviews=<file>     Deprecated reviews (article blacklist)
    --topics=<file>      Topical whitelist
    --journals=<file>    OA exceptions (journal whitelist)
    --publishers=<file>  Beall's list (publisher blacklist)
    --sparql=<file>      SPARQL pattern 
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_articles: String,
    flag_reviews: String,
    flag_topics: String,
    flag_journals: String,
    flag_publishers: String,
    flag_sparql: String,
    arg_query: String,
    cmd_run: bool,
    cmd_query: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    //println!("{:?}", args);
    if args.cmd_query {
        command_query(&args);
    }
    if args.cmd_run {
        command_run(&args);
    }
}
