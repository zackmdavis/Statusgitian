extern crate git2;
#[macro_use]
extern crate prettytable;

use std::env;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;

use git2::{Commit, Oid, Repository};
use prettytable::Table;
use prettytable::format;

trait Analyzer {
    fn register_commit(&mut self, commit: &Commit) -> Result<(), Box<Error>>;
    fn report(&self);
}

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

#[derive(Debug, Default)]
struct Counter(HashMap<String, Counts>);

impl Analyzer for Counter {
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

    fn report(&self) {
        let mut roster = self.0
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
}

#[derive(Debug, Default)]
struct AuthorCommitterTimeGapFinder(BTreeMap<i64, Oid>);

impl Analyzer for AuthorCommitterTimeGapFinder {
    fn register_commit(&mut self, commit: &Commit) -> Result<(), Box<Error>> {
        let authorship_timestamp = commit.author().when().seconds();
        let commit_timestamp = commit.committer().when().seconds();
        let gap = commit_timestamp - authorship_timestamp;
        if gap != 0 {
            // XXX: this overwrites duplicate gaps
            self.0.insert(gap, commit.id());
        }
        Ok(())
    }

    fn report(&self) {
        // TODO: pretty table
        println!("{:?}", self);
    }
}

fn analyze_repo(
    path: &str,
    analyzers: &mut [Box<Analyzer>],
) -> Result<(), Box<Error>> {
    let repo = Repository::open(path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;
        for analyzer in analyzers.iter_mut() {
            analyzer.register_commit(&commit)?;
        }
    }
    Ok(())
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let mut analyzers: Vec<Box<Analyzer>> = vec![
        Box::new(Counter::default()),
        Box::new(AuthorCommitterTimeGapFinder::default()),
    ];
    analyze_repo(&args[1], &mut analyzers).expect("couldn't analyze");
    for analyzer in analyzers {
        analyzer.report();
    }
}
