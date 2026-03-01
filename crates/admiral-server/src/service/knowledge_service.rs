//! Knowledge RAG: ingest Markdown from a directory, query by full-text search.

use crate::error::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeSnippet {
    pub content: String,
    pub source_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Query knowledge base using PostgreSQL full-text search. Returns top_k snippets.
pub async fn query(
    db: &sea_orm::DatabaseConnection,
    query_text: &str,
    top_k: usize,
) -> Result<Vec<KnowledgeSnippet>> {
    let top_k = top_k.min(20).max(1);
    if query_text.trim().is_empty() {
        return Ok(Vec::new());
    }
    // Escape single quotes in query for plainto_tsquery (safe: we only pass one param).
    let q = query_text.replace('\'', "''");
    let sql = format!(
        r#"SELECT content, source_file, title FROM knowledge_chunks
           WHERE to_tsvector('simple', content) @@ plainto_tsquery('simple', $1)
           ORDER BY ts_rank_cd(to_tsvector('simple', content), plainto_tsquery('simple', $1)) DESC
           LIMIT {}"#,
        top_k
    );
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        &sql,
        [sea_orm::Value::String(Some(Box::new(q)))],
    );
    let rows = db.query_all(stmt).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let content: String = row.try_get_by_index(0)?;
        let source_file: String = row.try_get_by_index(1)?;
        let title: Option<String> = row.try_get_by_index(2)?;
        out.push(KnowledgeSnippet {
            content,
            source_file,
            title,
        });
    }
    Ok(out)
}

/// Chunk strategy: split by Markdown ## headers, then by size if needed.
fn chunk_markdown(content: &str, _source_file: &str) -> Vec<(String, Option<String>, usize)> {
    let mut chunks = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current = String::new();
    const MAX_CHARS: usize = 2000;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            if !current.is_empty() {
                let text = current.trim().to_string();
                if !text.is_empty() {
                    chunks.push((text, current_title.clone(), chunks.len()));
                }
                current.clear();
            }
            current_title = Some(trimmed.trim_start_matches("## ").to_string());
            current.push_str(line);
            current.push('\n');
        } else if trimmed.starts_with("# ") && current_title.is_none() {
            if !current.is_empty() {
                let text = current.trim().to_string();
                if !text.is_empty() {
                    chunks.push((text, current_title.clone(), chunks.len()));
                }
                current.clear();
            }
            current_title = Some(trimmed.trim_start_matches("# ").to_string());
            current.push_str(line);
            current.push('\n');
        } else {
            current.push_str(line);
            current.push('\n');
            if current.len() >= MAX_CHARS {
                let text = current.trim().to_string();
                if !text.is_empty() {
                    chunks.push((text, current_title.clone(), chunks.len()));
                }
                current.clear();
            }
        }
    }
    if !current.is_empty() {
        let text = current.trim().to_string();
        if !text.is_empty() {
            chunks.push((text, current_title, chunks.len()));
        }
    }
    chunks
}

/// Collect all .md files under dir recursively. Returns (relative_path, absolute_path).
fn collect_md_files(dir: &Path) -> std::io::Result<Vec<(String, std::path::PathBuf)>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            out.extend(collect_md_files(&path)?);
        } else if path.extension().map_or(false, |e| e == "md") {
            let rel = path.strip_prefix(dir).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| path.file_name().unwrap_or_default().to_string_lossy().to_string());
            out.push((rel, path));
        }
    }
    Ok(out)
}

/// Ingest all Markdown files from a directory (recursive) into knowledge_chunks.
/// Replaces existing chunks for the same source_file. Returns number of chunks inserted.
pub async fn ingest_from_dir(db: &sea_orm::DatabaseConnection, dir: &Path) -> Result<u32> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let files = collect_md_files(dir).map_err(anyhow::Error::msg)?;
    let mut total = 0u32;
    for (source_file, path) in files {
        let content = fs::read_to_string(&path).map_err(anyhow::Error::msg)?;
        let chunks = chunk_markdown(&content, &source_file);
        if chunks.is_empty() {
            continue;
        }
        let del_sql = "DELETE FROM knowledge_chunks WHERE source_file = $1";
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            del_sql,
            [sea_orm::Value::String(Some(Box::new(source_file.clone())))],
        ))
        .await?;

        for (content, title, chunk_index) in chunks {
            let id = Uuid::new_v4();
            let metadata = "{}";
            let ins_sql = r#"INSERT INTO knowledge_chunks (id, source_file, chunk_index, title, content, metadata, created_at, updated_at)
                            VALUES ($1, $2, $3, $4, $5, $6::jsonb, NOW(), NOW())"#;
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                ins_sql,
                [
                    sea_orm::Value::Uuid(Some(Box::new(id))),
                    sea_orm::Value::String(Some(Box::new(source_file.clone()))),
                    sea_orm::Value::Int(Some(chunk_index as i32)),
                    sea_orm::Value::String(title.map(Box::new)),
                    sea_orm::Value::String(Some(Box::new(content))),
                    sea_orm::Value::String(Some(Box::new(metadata.to_string()))),
                ],
            ))
            .await?;
            total += 1;
        }
    }
    Ok(total)
}
