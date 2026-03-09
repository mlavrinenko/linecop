use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::{Config, CountMode, Override};
use crate::counter::FileStats;

pub fn make_file(path: &str, lang: &str, code: u64, comments: u64, blanks: u64) -> FileStats {
    FileStats {
        path: PathBuf::from(path),
        language: lang.to_owned(),
        total: code + comments + blanks,
        code,
        comments,
        blanks,
    }
}

pub fn make_config(limits: &[(&str, u64)], overrides: Vec<Override>, mode: CountMode) -> Config {
    Config {
        count_mode: mode,
        limits: limits
            .iter()
            .map(|(kk, vv)| ((*kk).to_owned(), *vv))
            .collect::<BTreeMap<_, _>>(),
        overrides,
        exclude_dirs: vec!["target".to_owned()],
    }
}
