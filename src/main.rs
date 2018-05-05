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

// TODO: trait that generalizes the wordcount and author–commit-gap state
// structures; any given revwalk on a repository can take an arbitrary vector
// of these trait objects and call `register_commit` on all of them for each
// commit

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

#[derive(Debug, Default)]
struct AuthorCommitterTimeGapFinder(BTreeMap<i64, Oid>);

impl AuthorCommitterTimeGapFinder {
    fn register_commit(&mut self, commit: &Commit) -> Result<(), Box<Error>> {
        let authorship_timestamp = commit.author().when().seconds();
        let commit_timestamp = commit.committer().when().seconds();
        let gap = commit_timestamp - authorship_timestamp;
        if gap != 0 {
            self.0.insert(gap, commit.id());
        }
        Ok(())
    }
}

fn analyze_repo(
    path: &str,
) -> Result<(Counter, AuthorCommitterTimeGapFinder), Box<Error>> {
    let mut counter = Counter::default();
    let mut gapfinder = AuthorCommitterTimeGapFinder::default();
    let repo = Repository::open(path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;
        counter.register_commit(&commit)?;
        gapfinder.register_commit(&commit)?;
    }
    Ok((counter, gapfinder))
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let (counter, gapfinder) =
        analyze_repo(&args[1]).expect("couldn't analyze");
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

    // TODO: pretty table
    println!("{:?}", gapfinder);
}
