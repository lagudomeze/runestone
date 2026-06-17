use std::path::Path;

use rig::embeddings::EmbeddingModel;

use crate::{
    error::{IntoExn, Result},
    extractor::FileEntry,
    memory::MemoryHit,
};

/// In-memory vector index over all `.md` files under `owner_root`.
pub(crate) struct Index {
    entries: Vec<IndexEntry>,
    embedder: Option<Embedder>,
}

struct IndexEntry {
    path: String,
    content: String,
    is_abstract: bool,
    vector: Option<Vec<f32>>,
}

struct Embedder {
    model: rig_fastembed::EmbeddingModel,
}

impl Embedder {
    fn new() -> Option<Self> {
        let client = rig_fastembed::Client::new();
        let model = client.embedding_model(&rig_fastembed::FastembedModel::AllMiniLML6V2Q).ok()?;
        Some(Self { model })
    }

    async fn embed(&self, texts: &[String]) -> Vec<Vec<f32>> {
        match self.model.embed_texts(texts.iter().cloned()).await {
            Ok(embeddings) => embeddings
                .into_iter()
                .map(|e| e.vec.into_iter().map(|v| v as f32).collect())
                .collect(),
            Err(_) => vec![],
        }
    }
}

impl Index {
    pub async fn build(owner_root: &Path, data_dir: &Path) -> Self {
        let mut entries = Vec::new();
        let _ = collect_files(owner_root, data_dir, &mut entries);
        let embedder = Embedder::new();

        if let Some(ref e) = embedder {
            let texts: Vec<String> = entries.iter().map(|en| en.content.clone()).collect();
            let vectors = e.embed(&texts).await;
            for (entry, vec) in entries.iter_mut().zip(vectors) {
                entry.vector = Some(vec);
            }
        }

        Self { entries, embedder }
    }

    pub async fn search_async(&self, query: &str, limit: usize) -> Result<Vec<MemoryHit>> {
        if query.trim().is_empty() || self.entries.is_empty() {
            return Ok(vec![]);
        }

        if let Some(ref emb) = self.embedder {
            let q_vec =
                emb.embed(&[query.to_string()]).await.into_iter().next().unwrap_or_default();
            if !q_vec.is_empty() {
                return Ok(vector_search(&self.entries, &q_vec, limit));
            }
        }

        keyword_search(&self.entries, query, limit)
    }

    /// Directory-recursive retrieval:
    /// 1. Vector search L0 abstracts → candidates
    /// 2. LLM routes to relevant directories
    /// 3. For each relevant dir, collect L2 files
    /// 4. If dir has subdirs, recursively explore via L1 overview
    pub async fn recursive_search(
        &self,
        query: &str,
        limit: usize,
        _data_dir: &Path,
        extractor: impl crate::extractor::Extractor,
    ) -> Result<Vec<MemoryHit>> {
        // Step 1: vector search on L0 abstracts to get initial candidates
        let candidates = self.top_abstracts(query, 10).await;

        if candidates.is_empty() {
            return keyword_search(&self.entries, query, limit);
        }

        // Step 2: LLM routing — which directories are relevant?
        let dir_entries: Vec<FileEntry> = candidates
            .iter()
            .map(|(path, content)| {
                let name = std::path::Path::new(path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                FileEntry { name, content: content.clone() }
            })
            .collect();

        let relevant = extractor
            .route_directories(query, &dir_entries)
            .await
            .unwrap_or_else(|_| (0..dir_entries.len()).collect());

        // Step 3: collect L2 files from relevant directories
        let mut hits: Vec<MemoryHit> = Vec::new();
        for idx in relevant.iter().take(5) {
            if let Some((dir_path, _)) = candidates.get(*idx) {
                // Find all non-abstract files in or under this directory
                let dir_prefix = if dir_path.ends_with(".abstract.md") {
                    dir_path.trim_end_matches(".abstract.md")
                } else {
                    dir_path
                };
                for entry in &self.entries {
                    if !entry.is_abstract && entry.path.starts_with(dir_prefix) {
                        hits.push(MemoryHit {
                            path: entry.path.clone(),
                            snippet: entry.content.chars().take(300).collect(),
                            score: 0.8,
                        });
                    }
                }
            }
        }

        hits.truncate(limit);
        Ok(hits)
    }

    /// Get top N abstracts matching the query via vector or keyword search.
    async fn top_abstracts(&self, query: &str, n: usize) -> Vec<(String, String)> {
        let query_embed = if let Some(ref emb) = self.embedder {
            emb.embed(&[query.to_string()]).await.into_iter().next()
        } else {
            None
        };

        let abstracts: Vec<&IndexEntry> = self.entries.iter().filter(|e| e.is_abstract).collect();

        if let Some(q_vec) = query_embed {
            let mut scored: Vec<(f32, &IndexEntry)> = abstracts
                .iter()
                .filter(|e| e.vector.is_some())
                .map(|e| (cosine_similarity(&q_vec, e.vector.as_ref().unwrap()), *e))
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(n);
            scored.into_iter().map(|(_, e)| (e.path.clone(), e.content.clone())).collect()
        } else {
            abstracts.iter().take(n).map(|e| (e.path.clone(), e.content.clone())).collect()
        }
    }
}

fn vector_search(entries: &[IndexEntry], query_vec: &[f32], limit: usize) -> Vec<MemoryHit> {
    let mut scored: Vec<(f32, &IndexEntry)> = entries
        .iter()
        .filter(|e| e.vector.is_some())
        .map(|e| {
            let sim = cosine_similarity(query_vec, e.vector.as_ref().unwrap());
            (sim, e)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    scored
        .into_iter()
        .map(|(score, entry)| MemoryHit {
            path: entry.path.clone(),
            snippet: if entry.is_abstract {
                entry.content.clone()
            } else {
                entry.content.chars().take(200).collect()
            },
            score,
        })
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-10 || norm_b < 1e-10 { 0.0 } else { dot / (norm_a * norm_b) }
}

fn keyword_search(entries: &[IndexEntry], query: &str, limit: usize) -> Result<Vec<MemoryHit>> {
    let q = query.to_lowercase();
    let terms: Vec<&str> = q.split_whitespace().collect();

    let mut scored: Vec<(f32, &IndexEntry)> = entries
        .iter()
        .map(|e| {
            let lower = e.content.to_lowercase();
            let mut score = 0.0_f32;
            for t in &terms {
                let count = lower.matches(*t).count();
                if count > 0 {
                    score += 1.0 + (count as f32).ln_1p();
                }
            }
            if e.is_abstract {
                score *= 3.0;
            }
            (score, e)
        })
        .filter(|(s, _)| *s > 0.0)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    let max = scored.first().map(|s| s.0).unwrap_or(1.0);
    Ok(scored
        .into_iter()
        .map(|(score, e)| MemoryHit {
            path: e.path.clone(),
            snippet: if e.is_abstract {
                e.content.clone()
            } else {
                e.content.chars().take(200).collect()
            },
            score: if max > 0.0 { score / max } else { 0.0 },
        })
        .collect())
}

fn collect_files(dir: &Path, data_dir: &Path, out: &mut Vec<IndexEntry>) -> Result<()> {
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
            out.push(IndexEntry {
                path: rel.to_string_lossy().into_owned(),
                content,
                is_abstract,
                vector: None,
            });
        }
    }
    Ok(())
}
