use std::collections::HashSet;

use crate::commands::{CommandArgs, GitHubCli, ReferenceInput, ReferenceKind, ReferenceNumber};

const PARENT_STATUS_TITLE: &str = "### Parent Issue Status";

struct ParentCommentStats<'a> {
    total: usize,
    closed_count: usize,
    open_count: usize,
    open_lines: &'a str,
    parent_state: &'a str,
    strict_guard: bool,
}

pub(crate) struct ParentGuard {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
}

impl ParentGuard {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let strict_guard = self.resolve_strict_guard();

        if let Some(parent) = self.resolve_issue_number() {
            self.evaluate_parent_issue(&repo, &parent, strict_guard)?;
            return Ok(());
        }

        let child = self
            .resolve_child_number()
            .ok_or_else(|| "Missing --issue or invalid/missing --child".to_string())?;

        let mut seen = HashSet::new();

        for parent in self.read_direct_parent_numbers(&repo, &child) {
            if seen.insert(parent.as_string()) {
                self.evaluate_parent_issue(&repo, &parent, strict_guard)?;
            }
        }

        if seen.is_empty() {
            for parent in self.search_parent_numbers(&repo, &child)? {
                if parent.as_string() != child.as_string() && seen.insert(parent.as_string()) {
                    self.evaluate_parent_issue(&repo, &parent, strict_guard)?;
                }
            }
        }

        Ok(())
    }

    pub(crate) fn evaluate_parent_issue(
        &self,
        repo: &str,
        parent: &ReferenceNumber,
        strict_guard: bool,
    ) -> Result<(), String> {
        let parent_number = parent.as_string();
        let parent_details = match self.github.read_issue_details(repo, &parent_number)? {
            Some(details) => details,
            None => return Ok(()),
        };

        let mut child_refs = self.read_sub_issue_numbers(repo, parent);
        if child_refs.is_empty() {
            child_refs = self.extract_tasklist_refs(&parent_details.body);
        }
        if child_refs.is_empty() {
            return Ok(());
        }

        let total = child_refs.len();
        let mut closed_count = 0usize;
        let mut open_count = 0usize;
        let mut open_lines = String::new();

        for child in child_refs {
            let child_number = child.as_string();
            match self.github.read_issue_details(repo, &child_number)? {
                Some(details) if details.state == "CLOSED" => {
                    closed_count += 1;
                }
                Some(details) => {
                    open_count += 1;
                    open_lines.push_str(&format!(
                        "- {} {}\n",
                        child.show_number(),
                        details.title.trim()
                    ));
                }
                None => {
                    open_count += 1;
                    open_lines.push_str(&format!(
                        "- {} (unreadable or missing)\n",
                        child.show_number()
                    ));
                }
            }
        }

        let comment = self.build_parent_comment(
            parent,
            ParentCommentStats {
                total,
                closed_count,
                open_count,
                open_lines: &open_lines,
                parent_state: parent_details.state.as_str(),
                strict_guard,
            },
        );

        self.github.upsert_issue_comment_by_marker(
            repo,
            &parent_number,
            &format!("<!-- parent-issue-status:{parent_number} -->"),
            &comment,
        )?;

        if open_count == 0 && parent_details.state == "OPEN" {
            self.github.close_issue(
                repo,
                &parent_number,
                "completed",
                "All required child issues are closed. Auto-closed by parent-issue-guard.",
            )?;
        }

        if strict_guard && parent_details.state == "CLOSED" && open_count != 0 {
            self.github.reopen_issue(repo, &parent_number)?;
        }

        Ok(())
    }

    fn resolve_issue_number(&self) -> Option<ReferenceNumber> {
        self.args
            .extract_reference_number("--issue", Some(ReferenceKind::Issue))
    }

    fn resolve_child_number(&self) -> Option<ReferenceNumber> {
        self.args
            .extract_reference_number("--child", Some(ReferenceKind::Issue))
    }

    fn resolve_strict_guard(&self) -> bool {
        matches!(
            self.args
                .extract_flag_value("--strict-guard")
                .ok()
                .as_deref()
                .map(str::trim),
            Some("true" | "TRUE" | "True" | "1")
        )
    }

    fn read_direct_parent_numbers(
        &self,
        repo: &str,
        child: &ReferenceNumber,
    ) -> Vec<ReferenceNumber> {
        self.github
            .read_issue_parent_numbers(repo, &child.as_string())
            .unwrap_or_default()
    }

    fn read_sub_issue_numbers(&self, repo: &str, parent: &ReferenceNumber) -> Vec<ReferenceNumber> {
        self.github
            .read_issue_sub_issue_numbers(repo, &parent.as_string())
            .unwrap_or_default()
    }

    fn search_parent_numbers(
        &self,
        repo: &str,
        child: &ReferenceNumber,
    ) -> Result<Vec<ReferenceNumber>, String> {
        self.github
            .search_issue_numbers(&format!("repo:{repo} is:issue \"{}\"", child.show_number()))
    }

    fn extract_tasklist_refs(&self, body: &str) -> Vec<ReferenceNumber> {
        let mut seen = HashSet::new();
        let mut references = Vec::new();

        for line in body.lines() {
            let lower = line.to_ascii_lowercase();
            if !line.contains("- [") && !lower.contains("task") {
                continue;
            }

            if let Some(reference) = ReferenceInput::find_in_text(line, Some(ReferenceKind::Issue))
                .into_iter()
                .next()
            {
                let key = reference.as_string();
                if seen.insert(key) {
                    references.push(reference);
                }
            }
        }

        references
    }

    fn build_parent_comment(
        &self,
        parent: &ReferenceNumber,
        stats: ParentCommentStats<'_>,
    ) -> String {
        let mut comment = format!(
            "<!-- parent-issue-status:{} -->\n{}\nParent: {}\n\n- Required children detected: {}\n- Closed: {}\n- Open: {}\n\n",
            parent.as_string(),
            PARENT_STATUS_TITLE,
            parent.show_number(),
            stats.total,
            stats.closed_count,
            stats.open_count
        );

        if stats.open_count == 0 {
            comment.push_str("All required child issues are closed. This parent can be closed.");
        } else {
            comment.push_str("Some required child issues are still open:\n");
            comment.push_str(stats.open_lines);
            if stats.strict_guard && stats.parent_state == "CLOSED" {
                comment.push_str(
                    "\nGuard action: parent was reopened because required children are still open.",
                );
            }
        }

        comment
    }
}
