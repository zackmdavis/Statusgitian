extern crate git2;
#[macro_use]
extern crate prettytable;

use std::env;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;

use git2::{Commit, Repository};
use prettytable::Table;
use prettytable::format;

#[derive(Debug, Default)]
struct Counts {
    commits: usize,
    words: usize,
}

impl Counts {
    fn words_per_commit(&self) -> f32 {
        (self.words as f32) / (self.commits as f32)
    }
}

#[derive(Debug)]
struct Counter(HashMap<String, Counts>);

impl Counter {
    fn register_commit(&mut self, commit: &Commit) -> Result<(), Box<Error>> {
        let author =
            commit.author().name().ok_or("malformed name")?.to_owned();
        let wordcount = commit
            .message()
            .ok_or("malformed message")?
            .split_whitespace()
            .count();
        let counts = self.0.entry(author).or_insert(Counts::default());
        counts.commits += 1;
        counts.words += wordcount;
        Ok(())
    }
}

fn wordcount_repo(path: &str) -> Result<Counter, Box<Error>> {
    let mut counter = Counter(HashMap::new());
    let repo = Repository::open(path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;
        counter.register_commit(&commit)?;
    }
    Ok(counter)
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let counter = wordcount_repo(&args[1]).expect("couldn't count");
    let mut roster = counter
        .0
        .iter()
        .map(|(name, counts)| {
            (
                name,
                counts.commits,
                counts.words,
                counts.words_per_commit(),
            )
        })
        .collect::<Vec<_>>();
    roster.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(Ordering::Less));

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row![
        "author",
        "total commits",
        "total words",
        "words / commit"
    ]);
    for (name, total_commits, total_words, words_per_commit) in roster {
        table.add_row(row![
            name,
            total_commits,
            total_words,
            words_per_commit
        ]);
    }
    table.printstd();
}
