use git2::{
    Commit, Cred, Direction, Error, ObjectType, Oid, ProxyOptions, RemoteCallbacks, Repository,
    Signature,
};
use relative_path::RelativePath;
use std::env;
use std::fs::canonicalize;
use std::ops::FnMut;
use std::path::{Path, PathBuf};
use std::process;
use time::Timespec;

fn open_repo(repo_path: &str) -> Repository {
    let repo = match Repository::open(repo_path) {
        Ok(repo) => repo,
        Err(e) => panic!("ERROR, failed to open repo {}", e),
    };
    println!("{} state={:?}", repo.path().display(), repo.state());
    repo
}

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("ERROR, couldn't find last commit"))
}

fn display_commit(commit: &Commit) {
    let timestamp = commit.time().seconds();
    let tm = time::at(time::Timespec::new(timestamp, 0));
    println!(
        "commit {}\nAuthor: {}\nDate: {}\n\n  {}",
        commit.id(),
        commit.author(),
        tm.rfc822(),
        commit.message().unwrap_or("No commit message")
    );
}

fn add_and_commit(msg: &str, path: &Path, repo: &Repository) -> Result<Oid, git2::Error> {
    let mut index = repo.index().unwrap();
    index.add_path(path).unwrap();
    let oid = index.write_tree().unwrap();
    let signature = Signature::now("Skyler Favors", "sky86one@gmail.com").unwrap();
    let parent_commit = find_last_commit(repo).unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        msg,
        &tree,
        &[&parent_commit],
    )
}

fn push(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
    with_credentials(repo, |cred_callback| {
        let mut remote = repo.find_remote("origin")?;
        let mut cb = RemoteCallbacks::new();
        let mut options = git2::PushOptions::new();

        cb.credentials(cred_callback);
        options.remote_callbacks(cb);

        remote.push(
            &[format!(
                "refs/heads/{}:refs/heads/{}",
                branch_name, branch_name
            )],
            Some(&mut options),
        )?;

        Ok(())
    })
}

fn with_credentials<F>(repo: &Repository, mut f: F) -> Result<(), git2::Error>
where
    F: FnMut(&mut git2::Credentials) -> Result<(), git2::Error>,
{
    let config = repo.config()?;

    let mut tried_sshkey = false;
    let mut tried_cred_helper = false;
    let mut tried_default = false;

    f(&mut |url, username, allowed| {
        if allowed.contains(git2::CredentialType::USERNAME) {
            return Err(git2::Error::from_str("No username specified in remote URL"));
        }
        if allowed.contains(git2::CredentialType::SSH_KEY) && !tried_sshkey {
            tried_sshkey = true;
            let username = username.unwrap();
            return git2::Cred::ssh_key_from_agent(username);
        }

        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) && !tried_cred_helper {
            tried_cred_helper = true;
            return git2::Cred::credential_helper(&config, url, username);
        }

        if allowed.contains(git2::CredentialType::DEFAULT) && !tried_default {
            tried_default = true;
            return git2::Cred::default();
        }

        Err(git2::Error::from_str("No auth methods succeeded"))
    })
}

fn main() {
    let repo = open_repo("/Users/sfav/Projects/ideas/");

    let path = Path::new("README.md");

    let add = add_and_commit("Hello world1", path, &repo).unwrap();

    let commit = find_last_commit(&repo).expect("Couldnt find commit");
    display_commit(&commit);
    let remote = repo.find_remote("origin").unwrap();

    println!("pushing to {}", remote.url().unwrap());
    let r = push(&repo, "main").expect("ERROR, pushign");
}
