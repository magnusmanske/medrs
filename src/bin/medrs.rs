extern crate config;
extern crate mediawiki;
extern crate papers;
extern crate regex;
extern crate wikibase;
#[macro_use]
extern crate lazy_static;

/*
use papers::crossref2wikidata::Crossref2Wikidata;
use papers::orcid2wikidata::Orcid2Wikidata;
use papers::pubmed2wikidata::Pubmed2Wikidata;
use papers::semanticscholar2wikidata::Semanticscholar2Wikidata;
*/
use docopt::Docopt;
use mediawiki::api::Api;
use papers::wikidata_papers::WikidataPapers;
use papers::*;
use regex::Regex;
use serde::Deserialize;
use std::str;
use std::{
    fs::File,
    io::{prelude::*, BufReader},
};
use urlencoding;

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

fn get_api_url_for_wiki(wiki: &String) -> Option<String> {
    // Get site matrix from wikidata
    let api = Api::new("https://www.wikidata.org/w/api.php").expect("Can't connect to Wikidata");
    let params = api.params_into(&vec![("action", "sitematrix")]);
    let site_matrix = api
        .get_query_api_json(&params)
        .expect("Can't load sitematrix from wikidata API");
    //println!("{:#?}", &site_matrix);

    // Go through the "normal" objects
    let mut ret: Option<String> = None;
    site_matrix["sitematrix"]
        .as_object()
        .expect("sitematrix is not an object")
        .iter()
        .for_each(|(_, data)| {
            match data["site"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|x| {
                    if x["dbname"].as_str().unwrap_or("") == wiki {
                        x["url"].as_str()
                    } else {
                        None
                    }
                })
                .next()
            {
                Some(url) => {
                    ret = Some(url.to_string() + "/w/api.php");
                }
                None => {}
            }
        });

    // Try the "specials"
    site_matrix["sitematrix"]["specials"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .for_each(|x| {
            if x["dbname"].as_str().unwrap_or("") == wiki {
                ret = Some(x["url"].as_str().unwrap_or("").to_string() + "/w/api.php");
            }
        });

    ret
}

fn get_external_urls(api_url: &String, title: &String) -> Vec<String> {
    let api = Api::new(&api_url).expect(&format!("Can't connect to {}", &api_url));
    let params = api.params_into(&vec![
        ("action", "query"),
        ("prop", "extlinks"),
        ("ellimit", "500"),
        ("titles", title.as_str()),
    ]);
    let result = api
        .get_query_api_json_all(&params)
        .expect("query.extlinks failed");
    let mut urls: Vec<String> = vec![];
    result["query"]["pages"]
        .as_object()
        .expect("query.pages in result not an object")
        .iter()
        .for_each(|(_page_id, data)| {
            data["extlinks"]
                .as_array()
                .expect("extlinks not an array")
                .iter()
                .for_each(|x| urls.push(x["*"].as_str().expect("* not a string").to_string()));
        });
    urls
}

fn get_paper_q(api: &Api, id: &GenericWorkIdentifier) -> Option<String> {
    let wdp = WikidataPapers::new();
    match &id.work_type {
        GenericWorkType::Property(prop) => {
            let result = wdp.search_external_id(&prop, &id.id, api);
            result.get(0).map(|s| s.to_owned()) // First one will do
        }
        _ => None,
    }

    /*
    wdp.add_adapter(Box::new(Pubmed2Wikidata::new()));
    wdp.add_adapter(Box::new(Crossref2Wikidata::new()));
    wdp.add_adapter(Box::new(Semanticscholar2Wikidata::new()));
    wdp.add_adapter(Box::new(Orcid2Wikidata::new()));

    let ids = vec![id.to_owned()];
    let ids = wdp.update_from_paper_ids(&ids);
    let q = ids
        .iter()
        .filter_map(|x| match x.work_type {
            GenericWorkType::Item => Some(x.id.to_owned()),
            _ => None,
        })
        .next();
    q*/
}

fn command_refs(args: &Args) {
    if args.arg_wiki.is_empty() {
        panic!("wiki code (e.g. 'enwiki') is required");
    }
    if args.arg_title.is_empty() {
        panic!("article title is required");
    }
    let wiki = &args.arg_wiki;
    let title = &args.arg_title;

    // Get the API URL for the wiki
    let api_url = match get_api_url_for_wiki(&wiki) {
        Some(url) => url,
        None => panic!("Can't find API URL for {}", &wiki),
    };

    // Get all external URLs from that page, on that wiki
    let urls = get_external_urls(&api_url, &title);
    //println!("{:#?}", &urls);
    lazy_static! {
        static ref RE_DOI: Regex = Regex::new(r#"^*.?//doi.org/(.+)$"#).unwrap();
        static ref RE_PMID: Regex =
            Regex::new(r#"^*.?//www.ncbi.nlm.nih.gov/pubmed/(\d+)$"#).unwrap();
        static ref RE_PMCID: Regex =
            Regex::new(r#"^*.?//www.ncbi.nlm.nih.gov/pmc/articles/PMC(\d+)$"#).unwrap();
    }

    let mut ids: Vec<GenericWorkIdentifier> = vec![];
    for url in urls {
        match RE_DOI.captures(&url) {
            Some(caps) => {
                let id = caps.get(1).unwrap().as_str();
                match urlencoding::decode(&id) {
                    Ok(id) => {
                        ids.push(GenericWorkIdentifier::new_prop(PROP_DOI, &id));
                    }
                    _ => {}
                }
            }
            None => {}
        }
        match RE_PMID.captures(&url) {
            Some(caps) => {
                let id = caps.get(1).unwrap().as_str();
                ids.push(GenericWorkIdentifier::new_prop(PROP_PMID, id));
            }
            None => {}
        }
        match RE_PMCID.captures(&url) {
            Some(caps) => {
                let id = caps.get(1).unwrap().as_str();
                ids.push(GenericWorkIdentifier::new_prop(PROP_PMCID, id));
            }
            None => {}
        }
    }

    let api = Api::new("https://www.wikidata.org/w/api.php").expect("Can't connect to Wikidata");
    for id in ids {
        match get_paper_q(&api, &id) {
            Some(q) => {
                println!("{}", &q);
            }
            None => {
                /*
                /TODO
                let prop = match &id.work_type {
                    GenericWorkType::Property(p) => p,
                    _ => continue,
                };
                println!("No item for https://www.wikidata.org/w/index.php?search=&search=haswbstatement%3A{}={}&title=Special%3ASearch&go=Go&ns0=1&ns120=1", &prop,&id.id);
                */
            }
        }
    }
}

const USAGE: &'static str = "
MEDRS

Usage:
    medrs run [--articles=<file>] [--reviews=<file>] [--topics=<file>] [--journals=<file>] [--publishers=<file>] [--sparql=<file>]
    medrs query <query>
    medrs refs <wiki> <title>
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
    arg_title: String,
    arg_wiki: String,
    cmd_run: bool,
    cmd_query: bool,
    cmd_refs: bool,
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
    if args.cmd_refs {
        command_refs(&args);
    }
}
