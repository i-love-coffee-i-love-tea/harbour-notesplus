//! Line-by-line diff engine for visual inspection and confirmation previews.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineType {
    Same,
    Add,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_line_no: Option<usize>,
    pub new_line_no: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSummary {
    pub additions: usize,
    pub deletions: usize,
    pub unchanged: usize,
    pub lines: Vec<DiffLine>,
}

impl DiffSummary {
    pub fn is_empty(&self) -> bool {
        self.additions == 0 && self.deletions == 0
    }
}

/// Computes a line-by-line diff between `old_text` and `new_text` using Longest Common Subsequence (LCS).
pub fn compute_line_diff(old_text: &str, new_text: &str) -> DiffSummary {
    let old_lines: Vec<&str> = if old_text.is_empty() {
        Vec::new()
    } else {
        old_text.lines().collect()
    };

    let new_lines: Vec<&str> = if new_text.is_empty() {
        Vec::new()
    } else {
        new_text.lines().collect()
    };

    let n = old_lines.len();
    let m = new_lines.len();

    // Guard against excessive memory allocation for very large inputs
    if n > 0 && m > 0 && n.saturating_mul(m) > 5_000_000 {
        return DiffSummary {
            additions: 0,
            deletions: 0,
            unchanged: 0,
            lines: vec![DiffLine {
                line_type: DiffLineType::Same,
                content: format!("[Diff too large: {}x{} lines exceeds limit]", n, m),
                old_line_no: None,
                new_line_no: None,
            }],
        };
    }

    // LCS dynamic programming table
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if old_lines[i] == new_lines[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    // Backtrack to build diff
    let mut i = n;
    let mut j = m;
    let mut reversed_lines = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;
    let mut unchanged = 0;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            reversed_lines.push(DiffLine {
                line_type: DiffLineType::Same,
                content: old_lines[i - 1].to_string(),
                old_line_no: Some(i),
                new_line_no: Some(j),
            });
            unchanged += 1;
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            reversed_lines.push(DiffLine {
                line_type: DiffLineType::Add,
                content: new_lines[j - 1].to_string(),
                old_line_no: None,
                new_line_no: Some(j),
            });
            additions += 1;
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            reversed_lines.push(DiffLine {
                line_type: DiffLineType::Remove,
                content: old_lines[i - 1].to_string(),
                old_line_no: Some(i),
                new_line_no: None,
            });
            deletions += 1;
            i -= 1;
        }
    }

    reversed_lines.reverse();

    DiffSummary {
        additions,
        deletions,
        unchanged,
        lines: reversed_lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_diff() {
        let text = "= Title\nParagraph 1\nParagraph 2";
        let diff = compute_line_diff(text, text);
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 0);
        assert_eq!(diff.unchanged, 3);
        assert_eq!(diff.lines.len(), 3);
        assert!(diff.lines.iter().all(|l| l.line_type == DiffLineType::Same));
    }

    #[test]
    fn test_added_lines() {
        let old_text = "= Title\nLine 1";
        let new_text = "= Title\nLine 1\nLine 2";
        let diff = compute_line_diff(old_text, new_text);
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 0);
        assert_eq!(diff.lines.last().unwrap().line_type, DiffLineType::Add);
        assert_eq!(diff.lines.last().unwrap().content, "Line 2");
    }

    #[test]
    fn test_removed_lines() {
        let old_text = "= Title\nLine 1\nLine 2";
        let new_text = "= Title\nLine 2";
        let diff = compute_line_diff(old_text, new_text);
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 1);
        assert_eq!(diff.lines[1].line_type, DiffLineType::Remove);
        assert_eq!(diff.lines[1].content, "Line 1");
    }

    #[test]
    fn test_modified_lines() {
        let old_text = "= Title\n* [ ] Task 1";
        let new_text = "= Title\n* [x] Task 1";
        let diff = compute_line_diff(old_text, new_text);
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
        assert_eq!(diff.unchanged, 1);
    }

    #[test]
    fn test_empty_to_nonempty() {
        let diff = compute_line_diff("", "= New Document\nHello");
        assert_eq!(diff.additions, 2);
        assert_eq!(diff.deletions, 0);
    }

    #[test]
    fn test_rejects_excessively_large_input() {
        let old: String = (0..3000).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let new: String = (0..3000).map(|i| format!("changed {}", i)).collect::<Vec<_>>().join("\n");
        let result = compute_line_diff(&old, &new);
        assert_eq!(result.lines.len(), 1);
        assert!(result.lines[0].content.contains("Diff too large"));
    }
}
