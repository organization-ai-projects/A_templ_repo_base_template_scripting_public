use crate::commands::GitCli;
use crate::regex::{
    BREAKING_CHANGE_CHECKBOX_REGEX, BREAKING_CHANGE_LABEL_REGEX, BREAKING_CHANGE_NEGATIVE_REGEX,
    BREAKING_CONVENTIONAL_COMMIT_REGEX, BREAKING_SCOPE_SUBJECT_REGEX,
};

#[derive(Default)]
pub(crate) struct BreakingChangeAnalysis {
    is_breaking: bool,
    scopes: Vec<String>,
    commits: Vec<String>,
}

impl BreakingChangeAnalysis {
    pub(crate) fn analyze(
        git: &GitCli,
        worktree: &str,
        base_ref: &str,
        head_ref: &str,
    ) -> Result<Self, String> {
        let range = format!("origin/{base_ref}..origin/{head_ref}");
        let commit_records = git.log_commit_records(worktree, &range)?;
        let mut analysis = Self::default();

        for (full_hash, subject, body) in commit_records {
            let combined_text = if body.is_empty() {
                subject.clone()
            } else {
                format!("{subject}\n{body}")
            };

            if !Self::text_indicates_breaking_change(&combined_text) {
                continue;
            }

            analysis.is_breaking = true;
            let short_hash = full_hash.chars().take(7).collect::<String>();
            if !short_hash.is_empty() && !analysis.commits.contains(&short_hash) {
                analysis.commits.push(short_hash);
            }

            for scope in Self::extract_breaking_scopes_from_subject(&subject) {
                if !analysis.scopes.contains(&scope) {
                    analysis.scopes.push(scope);
                }
            }
        }

        if analysis.is_breaking && analysis.scopes.is_empty() {
            let files = git.diff_name_only(worktree, &range)?;
            let mut inferred_scopes = Self::infer_scopes_from_files(&files);
            inferred_scopes.sort();
            inferred_scopes.dedup();
            analysis.scopes = inferred_scopes;
        }

        Ok(analysis)
    }

    pub(crate) fn is_breaking(&self) -> bool {
        self.is_breaking
    }

    pub(crate) fn scopes(&self) -> &[String] {
        &self.scopes
    }

    pub(crate) fn commits(&self) -> &[String] {
        &self.commits
    }

    fn text_indicates_breaking_change(text: &str) -> bool {
        let (
            Ok(negative_pattern),
            Ok(checkbox_pattern),
            Ok(label_pattern),
            Ok(conventional_pattern),
        ) = (
            &*BREAKING_CHANGE_NEGATIVE_REGEX,
            &*BREAKING_CHANGE_CHECKBOX_REGEX,
            &*BREAKING_CHANGE_LABEL_REGEX,
            &*BREAKING_CONVENTIONAL_COMMIT_REGEX,
        )
        else {
            return false;
        };

        if negative_pattern.is_match(text) {
            return false;
        }

        checkbox_pattern.is_match(text)
            || label_pattern.is_match(text)
            || text
                .lines()
                .next()
                .is_some_and(|line| conventional_pattern.is_match(line))
    }

    fn extract_breaking_scopes_from_subject(subject: &str) -> Vec<String> {
        let Ok(pattern) = &*BREAKING_SCOPE_SUBJECT_REGEX else {
            return Vec::new();
        };
        let Some(captures) = pattern.captures(subject) else {
            return Vec::new();
        };

        captures[1]
            .split(',')
            .map(str::trim)
            .filter(|scope| !scope.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    fn infer_scopes_from_files(files: &[String]) -> Vec<String> {
        let mut scopes = Vec::new();

        for file in files {
            if Self::is_root_crate_file(file) {
                scopes.push("repo_base_template_scripting_public".to_string());
                continue;
            }

            if let Some(scope) = Self::infer_scope_from_path(file) {
                scopes.push(scope);
            }
        }

        scopes
    }

    fn is_root_crate_file(file: &str) -> bool {
        file == "Cargo.toml"
            || file == "Cargo.lock"
            || file.starts_with("src/")
            || file.starts_with(".cargo/")
            || file.starts_with("rust-toolchain")
    }

    fn infer_scope_from_path(file: &str) -> Option<String> {
        if let Some(prefix) = file.strip_suffix("/Cargo.toml") {
            return prefix
                .rsplit('/')
                .next()
                .filter(|segment| !segment.is_empty())
                .map(ToString::to_string);
        }

        if let Some((prefix, _)) = file.split_once("/src/") {
            return prefix
                .rsplit('/')
                .next()
                .filter(|segment| !segment.is_empty())
                .map(ToString::to_string);
        }

        None
    }
}
