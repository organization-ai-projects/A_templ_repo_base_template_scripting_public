use std::process::{Command, Output};

use serde_json::Value;

use crate::commands::{ReferenceInput, ReferenceKind, ReferenceNumber};

pub(crate) struct GitHubCli;

pub(crate) struct IssueDetails {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) state: String,
}

pub(crate) struct MilestoneSummary {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) open_issues: u64,
}

pub(crate) enum PullRequestCiStatus {
    Unknown,
    Fail,
    Running,
    Pass,
}

impl PullRequestCiStatus {
    pub(crate) fn as_label(&self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN ⚪",
            Self::Fail => "FAIL ❌",
            Self::Running => "RUNNING ⏳",
            Self::Pass => "PASS ✅",
        }
    }
}

impl GitHubCli {
    pub(crate) fn execute_api(&self, args: &[&str]) -> Result<Output, String> {
        let mut gh_args = vec!["api"];
        gh_args.extend_from_slice(args);
        self.execute_gh(&gh_args)
    }

    pub(crate) fn execute_command(&self, args: &[&str]) -> Result<Output, String> {
        self.execute_gh(args)
    }

    pub(crate) fn read_api(&self, args: &[&str]) -> Result<String, String> {
        let output = self.execute_api(args)?;
        self.require_success(&self.with_api_prefix(args), output)
    }

    pub(crate) fn read_command(&self, args: &[&str]) -> Result<String, String> {
        let output = self.execute_command(args)?;
        self.require_success(args, output)
    }

    pub(crate) fn api_succeeded(&self, args: &[&str]) -> Result<bool, String> {
        let output = self.execute_api(args)?;
        Ok(output.status.success())
    }

    pub(crate) fn read_optional_reference_command(
        &self,
        args: &[&str],
        kind: Option<ReferenceKind>,
    ) -> Result<Option<ReferenceNumber>, String> {
        let output = self.read_command(args)?;
        let trimmed = output.trim();

        if trimmed.is_empty() || trimmed == "null" {
            return Ok(None);
        }

        Ok(ReferenceInput::from_text(trimmed)
            .with_kind_opt(kind)
            .normalize())
    }

    pub(crate) fn qualify_reference(
        &self,
        repo: &str,
        reference: ReferenceNumber,
    ) -> Result<Option<ReferenceNumber>, String> {
        let number = reference.as_string();
        let is_pull_request = self.api_succeeded(&[&format!("repos/{repo}/pulls/{number}")])?;

        if is_pull_request {
            return Ok(Some(reference.with_kind(ReferenceKind::PullRequest)));
        }

        let is_issue = self.api_succeeded(&[&format!("repos/{repo}/issues/{number}")])?;

        if is_issue {
            Ok(Some(reference.with_kind(ReferenceKind::Issue)))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn read_branch_sha(&self, repo: &str, branch: &str) -> Result<String, String> {
        self.read_api(&[
            &format!("repos/{repo}/git/ref/heads/{branch}"),
            "--jq",
            ".object.sha",
        ])
    }

    pub(crate) fn branch_exists(&self, repo: &str, branch: &str) -> Result<bool, String> {
        self.api_succeeded(&[&format!("repos/{repo}/git/ref/heads/{branch}")])
    }

    pub(crate) fn update_branch_ref(
        &self,
        repo: &str,
        branch: &str,
        sha: &str,
    ) -> Result<(), String> {
        self.read_api(&[
            "--method",
            "PATCH",
            &format!("repos/{repo}/git/refs/heads/{branch}"),
            "-f",
            &format!("sha={sha}"),
            "-F",
            "force=true",
        ])?;
        Ok(())
    }

    pub(crate) fn create_branch_ref(
        &self,
        repo: &str,
        branch: &str,
        sha: &str,
    ) -> Result<(), String> {
        self.read_api(&[
            "--method",
            "POST",
            &format!("repos/{repo}/git/refs"),
            "-f",
            &format!("ref=refs/heads/{branch}"),
            "-f",
            &format!("sha={sha}"),
        ])?;
        Ok(())
    }

    pub(crate) fn find_open_pull_request(
        &self,
        repo: &str,
        head: &str,
        base: &str,
    ) -> Result<Option<ReferenceNumber>, String> {
        self.read_optional_reference_command(
            &[
                "pr",
                "list",
                "--repo",
                repo,
                "--state",
                "open",
                "--head",
                head,
                "--base",
                base,
                "--json",
                "number",
                "--jq",
                ".[0].number",
            ],
            Some(ReferenceKind::PullRequest),
        )
    }

    pub(crate) fn create_pull_request(
        &self,
        repo: &str,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<String, String> {
        self.read_command(&[
            "pr", "create", "--repo", repo, "--base", base, "--head", head, "--title", title,
            "--body", body,
        ])
    }

    pub(crate) fn read_pull_request_number(
        &self,
        repo: &str,
        selector: &str,
    ) -> Result<Option<ReferenceNumber>, String> {
        self.read_optional_reference_command(
            &[
                "pr", "view", "--repo", repo, selector, "--json", "number", "--jq", ".number",
            ],
            Some(ReferenceKind::PullRequest),
        )
    }

    pub(crate) fn read_pull_request_field(
        &self,
        repo: &str,
        pr_number: &str,
        field: &str,
    ) -> Result<String, String> {
        self.read_command(&[
            "pr",
            "view",
            "--repo",
            repo,
            pr_number,
            "--json",
            field,
            "--jq",
            &format!(".{field}"),
        ])
    }

    pub(crate) fn read_pull_request_status_rollup(
        &self,
        repo: &str,
        pr_number: &str,
    ) -> Result<String, String> {
        self.read_command(&[
            "pr",
            "view",
            pr_number,
            "-R",
            repo,
            "--json",
            "statusCheckRollup",
            "--jq",
            ".statusCheckRollup[]? | [(.conclusion // \"\"),(.state // \"\"),(.status // \"\")] | @tsv",
        ])
    }

    pub(crate) fn read_pull_request_ci_status(
        &self,
        repo: &str,
        pr_number: &str,
    ) -> Result<PullRequestCiStatus, String> {
        let rollup = self.read_pull_request_status_rollup(repo, pr_number)?;

        if rollup.trim().is_empty() {
            return Ok(PullRequestCiStatus::Unknown);
        }

        let mut has_pass = false;

        for line in rollup.lines() {
            let mut parts = line.split('\t');
            let conclusion = parts.next().unwrap_or("");
            let state = parts.next().unwrap_or("");
            let status = parts.next().unwrap_or("");
            let raw = [conclusion, state, status]
                .into_iter()
                .find(|value| !value.is_empty())
                .unwrap_or("")
                .to_ascii_uppercase();

            match raw.as_str() {
                "FAILURE" | "FAILED" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED"
                | "STARTUP_FAILURE" => return Ok(PullRequestCiStatus::Fail),
                "IN_PROGRESS" | "QUEUED" | "PENDING" | "WAITING" | "REQUESTED" => {
                    return Ok(PullRequestCiStatus::Running);
                }
                "SUCCESS" | "PASSED" => has_pass = true,
                "" => return Ok(PullRequestCiStatus::Unknown),
                _ => {}
            }
        }

        if has_pass {
            Ok(PullRequestCiStatus::Pass)
        } else {
            Ok(PullRequestCiStatus::Unknown)
        }
    }

    pub(crate) fn update_pull_request_body(
        &self,
        repo: &str,
        pr_number: &str,
        body: &str,
    ) -> Result<(), String> {
        self.read_command(&["pr", "edit", pr_number, "-R", repo, "--body", body])?;
        Ok(())
    }

    pub(crate) fn read_pull_request_author_login(
        &self,
        repo: &str,
        pr_number: &str,
    ) -> Result<String, String> {
        self.read_command(&[
            "pr",
            "view",
            pr_number,
            "-R",
            repo,
            "--json",
            "author",
            "--jq",
            ".author.login // \"\"",
        ])
    }

    pub(crate) fn add_pull_request_labels(
        &self,
        repo: &str,
        pr_number: &str,
        labels: &[&str],
    ) -> Result<(), String> {
        let mut args = vec!["pr", "edit", "--repo", repo, pr_number];
        for label in labels {
            args.push("--add-label");
            args.push(label);
        }

        self.read_command(&args)?;
        Ok(())
    }

    pub(crate) fn merge_pull_request(
        &self,
        repo: &str,
        pr_number: &str,
        auto: bool,
    ) -> Result<(), String> {
        let mut args = vec!["pr", "merge", "--repo", repo, pr_number];

        if auto {
            args.push("--auto");
        }

        args.extend_from_slice(&["--merge", "--delete-branch"]);
        self.read_command(&args)?;
        Ok(())
    }

    pub(crate) fn read_issue_details(
        &self,
        repo: &str,
        issue_number: &str,
    ) -> Result<Option<IssueDetails>, String> {
        let output = self.execute_command(&[
            "issue",
            "view",
            issue_number,
            "-R",
            repo,
            "--json",
            "title,body,state",
        ])?;

        if !output.status.success() {
            return Ok(None);
        }

        let payload = self.parse_json(&output.stdout, "issue details")?;
        Ok(Some(IssueDetails {
            title: self.json_string(&payload, "title"),
            body: self.json_string(&payload, "body"),
            state: self.json_string(&payload, "state"),
        }))
    }

    pub(crate) fn list_open_issue_numbers(
        &self,
        repo: &str,
    ) -> Result<Vec<ReferenceNumber>, String> {
        self.read_references_from_command(
            &[
                "issue",
                "list",
                "-R",
                repo,
                "--state",
                "open",
                "--limit",
                "300",
                "--json",
                "number",
                "--jq",
                ".[].number",
            ],
            Some(ReferenceKind::Issue),
        )
    }

    pub(crate) fn read_issue_sub_issue_numbers(
        &self,
        repo: &str,
        issue_number: &str,
    ) -> Result<Vec<ReferenceNumber>, String> {
        let (owner, name) = self.split_repo(repo)?;
        self.read_references_from_api(
            &[
                "graphql",
                "-f",
                "query=query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){issue(number:$number){subIssues(first:100){nodes{number}}}}}",
                "-f",
                &format!("owner={owner}"),
                "-f",
                &format!("name={name}"),
                "-F",
                &format!("number={issue_number}"),
                "--jq",
                ".data.repository.issue.subIssues.nodes[]?.number",
            ],
            Some(ReferenceKind::Issue),
        )
    }

    pub(crate) fn read_issue_parent_numbers(
        &self,
        repo: &str,
        issue_number: &str,
    ) -> Result<Vec<ReferenceNumber>, String> {
        let (owner, name) = self.split_repo(repo)?;
        self.read_references_from_api(
            &[
                "graphql",
                "-f",
                "query=query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){issue(number:$number){parent{number}}}}",
                "-f",
                &format!("owner={owner}"),
                "-f",
                &format!("name={name}"),
                "-F",
                &format!("number={issue_number}"),
                "--jq",
                ".data.repository.issue.parent.number // empty",
            ],
            Some(ReferenceKind::Issue),
        )
    }

    pub(crate) fn search_issue_numbers(&self, query: &str) -> Result<Vec<ReferenceNumber>, String> {
        self.read_references_from_api(
            &[
                "search/issues",
                "-f",
                &format!("q={query}"),
                "--jq",
                ".items[].number",
            ],
            Some(ReferenceKind::Issue),
        )
        .map(|references| {
            references
                .into_iter()
                .filter(|reference| reference.is_issue())
                .collect::<Vec<_>>()
        })
        .or_else(|error| {
            if error.contains("gh [\"api\"") {
                Ok(Vec::new())
            } else {
                Err(error)
            }
        })
    }

    pub(crate) fn list_open_milestones(&self, repo: &str) -> Result<Vec<MilestoneSummary>, String> {
        let output = self.read_api(&[
            &format!("repos/{repo}/milestones?state=open"),
            "--paginate",
            "--jq",
            ".[] | [.number, (.title // \"\"), (.open_issues // 0)] | @tsv",
        ])?;

        let mut milestones = Vec::new();

        for line in output.lines() {
            let mut parts = line.split('\t');
            let Some(number) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
                continue;
            };
            let title = parts.next().unwrap_or("").to_string();
            let open_issues = parts
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0);

            milestones.push(MilestoneSummary {
                number,
                title,
                open_issues,
            });
        }

        Ok(milestones)
    }

    pub(crate) fn close_milestone(&self, repo: &str, milestone_number: u64) -> Result<(), String> {
        self.read_api(&[
            "-X",
            "PATCH",
            &format!("repos/{repo}/milestones/{milestone_number}"),
            "-f",
            "state=closed",
        ])?;
        Ok(())
    }

    pub(crate) fn upsert_issue_comment_by_marker(
        &self,
        repo: &str,
        issue_number: &str,
        marker: &str,
        body: &str,
    ) -> Result<(), String> {
        let comments = self.read_comment_summaries(repo, issue_number)?;

        if let Some(existing_id) = comments
            .into_iter()
            .find(|(_, comment_body)| comment_body.contains(marker))
            .map(|(id, _)| id)
        {
            self.read_api(&[
                "-X",
                "PATCH",
                &format!("repos/{repo}/issues/comments/{existing_id}"),
                "-f",
                &format!("body={body}"),
            ])?;
        } else {
            self.read_api(&[
                "-X",
                "POST",
                &format!("repos/{repo}/issues/{issue_number}/comments"),
                "-f",
                &format!("body={body}"),
            ])?;
        }

        Ok(())
    }

    pub(crate) fn close_issue(
        &self,
        repo: &str,
        issue_number: &str,
        reason: &str,
        comment: &str,
    ) -> Result<(), String> {
        self.read_command(&[
            "issue",
            "close",
            issue_number,
            "-R",
            repo,
            "--reason",
            reason,
            "--comment",
            comment,
        ])?;
        Ok(())
    }

    pub(crate) fn reopen_issue(&self, repo: &str, issue_number: &str) -> Result<(), String> {
        self.read_command(&["issue", "reopen", issue_number, "-R", repo])?;
        Ok(())
    }

    pub(crate) fn add_label_if_exists(
        &self,
        repo: &str,
        issue_number: &str,
        label: &str,
    ) -> Result<(), String> {
        if self.label_exists(repo, label)? {
            self.add_issue_label(repo, issue_number, label)?;
        }
        Ok(())
    }

    pub(crate) fn remove_label_if_exists(
        &self,
        repo: &str,
        issue_number: &str,
        label: &str,
    ) -> Result<(), String> {
        if self.label_exists(repo, label)? && self.issue_has_label(repo, issue_number, label)? {
            self.remove_issue_label(repo, issue_number, label)?;
        }
        Ok(())
    }

    pub(crate) fn label_exists(&self, repo: &str, label: &str) -> Result<bool, String> {
        let labels = self.read_command(&[
            "label", "list", "-R", repo, "--limit", "1000", "--json", "name", "--jq", ".[].name",
        ])?;

        Ok(labels.lines().any(|existing| existing.trim() == label))
    }

    pub(crate) fn issue_has_label(
        &self,
        repo: &str,
        issue_number: &str,
        label: &str,
    ) -> Result<bool, String> {
        let labels = self.read_command(&[
            "issue",
            "view",
            issue_number,
            "-R",
            repo,
            "--json",
            "labels",
            "--jq",
            ".labels[].name // \"\"",
        ])?;

        Ok(labels.lines().any(|existing| existing.trim() == label))
    }

    pub(crate) fn add_issue_label(
        &self,
        repo: &str,
        issue_number: &str,
        label: &str,
    ) -> Result<(), String> {
        self.read_command(&[
            "issue",
            "edit",
            issue_number,
            "-R",
            repo,
            "--add-label",
            label,
        ])?;
        Ok(())
    }

    pub(crate) fn remove_issue_label(
        &self,
        repo: &str,
        issue_number: &str,
        label: &str,
    ) -> Result<(), String> {
        self.read_command(&[
            "issue",
            "edit",
            issue_number,
            "-R",
            repo,
            "--remove-label",
            label,
        ])?;
        Ok(())
    }

    pub(crate) fn read_pull_request_commit_messages(
        &self,
        repo: &str,
        pr_number: &str,
    ) -> Result<Vec<String>, String> {
        let output = self.read_api(&[
            &format!("repos/{repo}/pulls/{pr_number}/commits"),
            "--paginate",
            "--jq",
            ".[].commit.message",
        ])?;

        Ok(output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect())
    }

    pub(crate) fn list_open_pull_request_rows(
        &self,
        repo: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let output = self.read_api(&[
            &format!("repos/{repo}/pulls?state=open&per_page=100"),
            "--paginate",
            "--jq",
            ".[] | [.number, (.body // \"\")] | @tsv",
        ])?;

        Ok(output
            .lines()
            .filter_map(|line| {
                let (number, body) = line.split_once('\t')?;
                Some((number.to_string(), body.to_string()))
            })
            .collect())
    }

    pub(crate) fn read_issue_assignee_logins(
        &self,
        repo: &str,
        issue_number: &str,
    ) -> Result<Vec<String>, String> {
        let output = self.read_command(&[
            "issue",
            "view",
            issue_number,
            "-R",
            repo,
            "--json",
            "assignees",
            "--jq",
            ".assignees[].login // \"\"",
        ])?;

        Ok(output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect())
    }

    pub(crate) fn query_issue_child_parent_relation(
        &self,
        repo: &str,
        child_issue_number: &str,
    ) -> Result<Value, String> {
        let (owner, name) = self.split_repo(repo)?;
        self.read_graphql_value(&[
            "-f",
            "query=query($owner:String!,$name:String!,$child:Int!){repository(owner:$owner,name:$name){child:issue(number:$child){id parent{number id}}}}",
            "-f",
            &format!("owner={owner}"),
            "-f",
            &format!("name={name}"),
            "-F",
            &format!("child={child_issue_number}"),
        ])
    }

    pub(crate) fn query_issue_parent_child_relation(
        &self,
        repo: &str,
        child_issue_number: &str,
        parent_issue_number: &str,
    ) -> Result<Value, String> {
        let (owner, name) = self.split_repo(repo)?;
        self.read_graphql_value(&[
            "-f",
            "query=query($owner:String!,$name:String!,$child:Int!,$parent:Int!){repository(owner:$owner,name:$name){child:issue(number:$child){id parent{number id}} parent:issue(number:$parent){id state}}}",
            "-f",
            &format!("owner={owner}"),
            "-f",
            &format!("name={name}"),
            "-F",
            &format!("child={child_issue_number}"),
            "-F",
            &format!("parent={parent_issue_number}"),
        ])
    }

    pub(crate) fn remove_sub_issue_relation(
        &self,
        parent_node_id: &str,
        child_node_id: &str,
    ) -> Result<Value, String> {
        self.read_graphql_value(&[
            "-f",
            "query=mutation($issueId:ID!,$subIssueId:ID!){removeSubIssue(input:{issueId:$issueId,subIssueId:$subIssueId}){issue{id}}}",
            "-f",
            &format!("issueId={parent_node_id}"),
            "-f",
            &format!("subIssueId={child_node_id}"),
        ])
    }

    pub(crate) fn add_sub_issue_relation(
        &self,
        parent_node_id: &str,
        child_node_id: &str,
    ) -> Result<Value, String> {
        self.read_graphql_value(&[
            "-f",
            "query=mutation($issueId:ID!,$subIssueId:ID!){addSubIssue(input:{issueId:$issueId,subIssueId:$subIssueId}){issue{subIssues(first:1){nodes{number}}}}}",
            "-f",
            &format!("issueId={parent_node_id}"),
            "-f",
            &format!("subIssueId={child_node_id}"),
        ])
    }

    fn execute_gh(&self, args: &[&str]) -> Result<Output, String> {
        Command::new("gh")
            .args(args)
            .output()
            .map_err(|error| format!("Failed to execute gh {:?}: {error}", args))
    }

    fn with_api_prefix<'a>(&self, args: &'a [&'a str]) -> Vec<&'a str> {
        let mut gh_args = vec!["api"];
        gh_args.extend_from_slice(args);
        gh_args
    }

    fn require_success(&self, args: &[&str], output: Output) -> Result<String, String> {
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(format!(
                "gh {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    fn read_references_from_command(
        &self,
        args: &[&str],
        kind: Option<ReferenceKind>,
    ) -> Result<Vec<ReferenceNumber>, String> {
        let output = self.read_command(args)?;
        Ok(self.parse_references(&output, kind))
    }

    fn read_references_from_api(
        &self,
        args: &[&str],
        kind: Option<ReferenceKind>,
    ) -> Result<Vec<ReferenceNumber>, String> {
        let output = self.read_api(args)?;
        Ok(self.parse_references(&output, kind))
    }

    fn parse_references(&self, output: &str, kind: Option<ReferenceKind>) -> Vec<ReferenceNumber> {
        output
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    ReferenceInput::from_text(trimmed)
                        .with_kind_opt(kind)
                        .normalize()
                }
            })
            .collect()
    }

    fn split_repo<'a>(&self, repo: &'a str) -> Result<(&'a str, &'a str), String> {
        repo.split_once('/')
            .ok_or_else(|| format!("Invalid repository format: {repo}"))
    }

    fn read_comment_summaries(
        &self,
        repo: &str,
        issue_number: &str,
    ) -> Result<Vec<(u64, String)>, String> {
        let output = self.read_api(&[
            &format!("repos/{repo}/issues/{issue_number}/comments"),
            "--paginate",
            "--jq",
            ".[] | {id: .id, body: (.body // \"\")} | @json",
        ])?;

        let mut comments = Vec::new();

        for line in output.lines() {
            let payload = self.parse_json(line.as_bytes(), "issue comment")?;
            let Some(id) = payload.get("id").and_then(Value::as_u64) else {
                continue;
            };
            let body = self.json_string(&payload, "body");
            comments.push((id, body));
        }

        Ok(comments)
    }

    fn parse_json(&self, input: &[u8], label: &str) -> Result<Value, String> {
        serde_json::from_slice(input)
            .map_err(|error| format!("Failed to parse {label} JSON: {error}"))
    }

    fn read_graphql_value(&self, args: &[&str]) -> Result<Value, String> {
        let mut graphql_args = vec!["graphql"];
        graphql_args.extend_from_slice(args);
        let output = self.execute_api(&graphql_args)?;

        if output.status.success() {
            self.parse_json(&output.stdout, "graphql response")
        } else {
            Err(format!(
                "gh {:?} failed: {}",
                self.with_api_prefix(&graphql_args),
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    fn json_string(&self, payload: &Value, key: &str) -> String {
        payload
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }
}
