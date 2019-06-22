Can run some canned SPARQL query with different lists injected. Should work for Linux and OSX.

## Step 1: [Install Rust](https://rustup.rs/) (it's a one-liner):


## Step 2(a): To just install the tool for yourself, run:
```
cargo install --git https://github.com/magnusmanske/medrs
```
This will download/compile for a while. After that, you can run `medrs` to see the options.

## Step 2(b): To get the source, and the test datafiles, run:
```
git clone https://github.com/magnusmanske/medrs
cd medrs
cargo build --release
```
This will download/compile for a while. After that, you can run `target/release/medrs` to see the options. You will have to use `./target/release/medrs` instead of just `medrs` in the following examples.

Some test files will be in the `data` directory. Example:
```
medrs run --articles data/articles --reviews data/reviews --topics data/topics --journals data/journals --publishers data/publishers --sparql data/sparql
```
The SPARQL query contains placeholders, see `data/sparql`.

## Data formats
Each of the data files you can pass is a list of Wikidata items (form Qxxx), one per line.
You can create these either manually, or from a SPARQL query:
```
medrs query 'SELECT ?article {?article wdt:P31 wd:Q45182324}'
```
This will run the SPARQL query, and output the items returned in the _first_ variable, one per line (you can then pipe the output into a file, and use those in `medrs run`).

## Extract from Wikipedia pages
You can get items for papers linked from a Wikipedia page using the `refs` command, giving the wiki and the page. Example:
```
medrs refs enwiki Lyme_disease
```
This will check all external links to doi.org, PubMed, and PubMedCentral. It will extract the respective IDs, and search for them on Wikidata. If an item is found, the item ID will be printed. This will take a minute or so for pages with lots of such links/references. You might want to store the output as a unique list, like so:
```
medrs refs enwiki Lyme_disease | sort -u > items.txt
```
These lists can be used for the `--articles` parameter in `medrs run`.
