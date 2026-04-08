use regex::Regex;

pub(crate) struct ManagedBodyBlock;

impl ManagedBodyBlock {
    pub(crate) fn remove(body: &str, start_marker: &str, end_marker: &str) -> String {
        let regex = Regex::new(&format!(
            r"\n?{}\n.*?\n{}\n?",
            regex::escape(start_marker),
            regex::escape(end_marker)
        ))
        .expect("managed body block regex must compile");

        let mut cleaned = regex.replace(body, "").to_string();
        while cleaned.contains("\n\n\n") {
            cleaned = cleaned.replace("\n\n\n", "\n\n");
        }
        cleaned.trim_end_matches('\n').to_string()
    }

    pub(crate) fn replace(
        body: &str,
        start_marker: &str,
        end_marker: &str,
        replacement: &str,
    ) -> String {
        let cleaned = Self::remove(body, start_marker, end_marker);
        if replacement.is_empty() {
            cleaned
        } else if cleaned.is_empty() {
            replacement.to_string()
        } else {
            format!("{cleaned}\n\n{replacement}")
        }
    }
}
