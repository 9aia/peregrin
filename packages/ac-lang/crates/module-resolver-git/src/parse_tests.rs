use crate::parse::parse_git_url;

#[test]
fn parse_basic_git_url() {
    let target =
        parse_git_url("git+https://github.com/org/repo.git@main/schemas/base.yml").unwrap();

    assert_eq!(target.repo_url, "https://github.com/org/repo.git");
    assert_eq!(target.reference, "main");
    assert_eq!(target.path, "schemas/base.yml");
}

#[test]
fn parse_git_url_with_tag() {
    let target =
        parse_git_url("git+https://github.com/org/repo.git@v1.0.0/configs/prod.toml").unwrap();

    assert_eq!(target.repo_url, "https://github.com/org/repo.git");
    assert_eq!(target.reference, "v1.0.0");
    assert_eq!(target.path, "configs/prod.toml");
}

#[test]
fn parse_git_url_with_commit_hash() {
    let target =
        parse_git_url("git+https://github.com/org/repo.git@abc123def/schema.json").unwrap();

    assert_eq!(target.repo_url, "https://github.com/org/repo.git");
    assert_eq!(target.reference, "abc123def");
    assert_eq!(target.path, "schema.json");
}

#[test]
fn parse_non_git_url_returns_none() {
    assert!(parse_git_url("https://github.com/org/repo.git@main/file.yml").is_none());
    assert!(parse_git_url("./base.yml").is_none());
    assert!(parse_git_url("").is_none());
}

#[test]
fn parse_git_url_missing_dot_git_returns_none() {
    assert!(parse_git_url("git+https://github.com/org/repo@main/file.yml").is_none());
}

#[test]
fn parse_git_url_no_path() {
    let target = parse_git_url("git+https://github.com/org/repo.git@main").unwrap();

    assert_eq!(target.repo_url, "https://github.com/org/repo.git");
    assert_eq!(target.reference, "main");
    assert_eq!(target.path, "");
}
