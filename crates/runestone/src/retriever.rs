use std::path::Path;

use crate::{
    error::{IntoExn, Result},
    memory::MemoryHit,
};

/// Search all `.md` files under `owner_root`. Prioritizes L0 `.abstract.md`
/// files (3x weight) over regular content files.
pub(crate) fn search(
    owner_root: &Path,
    data_dir: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<MemoryHit>> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut candidates = Vec::new();
    collect_files(owner_root, data_dir, &mut candidates)?;

    if candidates.is_empty() {
        return Ok(vec![]);
    }

    let query_terms: Vec<String> =
        query.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();

    let mut scored: Vec<(f32, String, String)> = Vec::new();
    for (path, content, is_abstract) in &candidates {
        let content_lower = content.to_lowercase();
        let mut score = 0.0_f32;

        for term in &query_terms {
            let count = content_lower.matches(term.as_str()).count();
            if count > 0 {
                score += 1.0 + (count as f32).ln_1p();
            }
        }

        if score > 0.0 {
            // Boost abstracts (L0 summaries are more information-dense)
            if *is_abstract {
                score *= 3.0;
            }

            let filename = std::path::Path::new(path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            for term in &query_terms {
                if filename.contains(term.as_str()) {
                    score += 2.0;
                }
            }

            let snippet = if *is_abstract {
                content.clone() // abstract is short, use full text
            } else {
                extract_snippet(&content_lower, &query_terms, 120)
            };
            scored.push((score, path.clone(), snippet));
        }
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    let max_score = scored.first().map(|s| s.0).unwrap_or(1.0);

    Ok(scored
        .into_iter()
        .map(|(score, path, snippet)| MemoryHit {
            path,
            snippet,
            score: if max_score > 0.0 { score / max_score } else { 0.0 },
        })
        .collect())
}

fn collect_files(dir: &Path, data_dir: &Path, out: &mut Vec<(String, String, bool)>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).into_exn()? {
        let entry = entry.into_exn()?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, data_dir, out)?;
        } else if path.extension().is_some_and(|e| e == "md")
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(rel) = path.strip_prefix(data_dir)
        {
            let is_abstract = path.file_name().map(|n| n == ".abstract.md").unwrap_or(false);
            out.push((rel.to_string_lossy().into_owned(), content, is_abstract));
        }
    }
    Ok(())
}

fn extract_snippet(content: &str, terms: &[String], max_len: usize) -> String {
    let mut best_pos = None;
    for term in terms {
        if let Some(pos) = content.find(term.as_str()) {
            best_pos = Some(pos);
            break;
        }
    }

    let Some(pos) = best_pos else {
        return content.chars().take(max_len).collect::<String>() + "...";
    };

    let start = pos.saturating_sub(max_len / 3);
    let start = content.floor_char_boundary(start);
    let end = (pos + max_len * 2 / 3).min(content.len());
    let end = content.ceil_char_boundary(end);
    let snippet: String = content[start..end].chars().collect();
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < content.len() { "..." } else { "" };
    format!("{prefix}{snippet}{suffix}")
}
