use crate::fetch::{GitFetcher, DEFAULT_DEPTH};
use crate::parse::GitTarget;

#[test]
fn default_depth_is_one() {
    assert_eq!(DEFAULT_DEPTH, 1);
}

#[test]
fn cache_path_is_deterministic() {
    let fetcher = GitFetcher::new(std::env::temp_dir(), DEFAULT_DEPTH);
    let target = GitTarget {
        repo_url: "https://github.com/org/repo.git".to_string(),
        reference: "main".to_string(),
        path: "schemas/base.yml".to_string(),
    };

    let path_a = fetcher.cache_path(&target);
    let path_b = fetcher.cache_path(&target);
    assert_eq!(path_a, path_b);
}

#[test]
fn cache_path_differs_for_different_refs() {
    let fetcher = GitFetcher::new(std::env::temp_dir(), DEFAULT_DEPTH);
    let target_a = GitTarget {
        repo_url: "https://github.com/org/repo.git".to_string(),
        reference: "main".to_string(),
        path: "schemas/base.yml".to_string(),
    };
    let target_b = GitTarget {
        repo_url: "https://github.com/org/repo.git".to_string(),
        reference: "dev".to_string(),
        path: "schemas/base.yml".to_string(),
    };

    assert_ne!(fetcher.cache_path(&target_a), fetcher.cache_path(&target_b));
}

#[test]
fn cache_path_differs_for_different_repos() {
    let fetcher = GitFetcher::new(std::env::temp_dir(), DEFAULT_DEPTH);
    let target_a = GitTarget {
        repo_url: "https://github.com/org/repo.git".to_string(),
        reference: "main".to_string(),
        path: "schemas/base.yml".to_string(),
    };
    let target_b = GitTarget {
        repo_url: "https://github.com/org/other.git".to_string(),
        reference: "main".to_string(),
        path: "schemas/base.yml".to_string(),
    };

    assert_ne!(fetcher.cache_path(&target_a), fetcher.cache_path(&target_b));
}
