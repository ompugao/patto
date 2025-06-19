use rusqlite::{Connection, params};
use std::collections::{HashMap, HashSet};
use sha2::{Digest, Sha256};
use chrono::Utc;
use anyhow::Result;

pub struct LineTracker {
    file_path: String,
    db_connection: Connection,
    line_ids: Vec<i64>,
}

impl LineTracker {
    pub fn new(file_path: &str) -> Result<Self> {
        let mut db_path = std::env::temp_dir(); 
        db_path.push("patto_line_tracker.db");
        let conn = Connection::open(&db_path)?;
        Self::init(file_path, conn)
    }

    pub fn new_in_memory(file_path: &str) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(file_path, conn)
    }

    fn init(file_path: &str, conn: Connection) -> Result<Self> {
        // Initialize database schema if needed
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS lines (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                current_content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                last_known_line_number INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_file_content ON lines (file_path, content_hash);
            CREATE INDEX IF NOT EXISTS idx_file_active ON lines (file_path, is_active);"
        )?;

        Ok(LineTracker {
            file_path: file_path.to_string(),
            db_connection: conn,
            line_ids: Vec::new(),
        })
    }

    pub fn process_file_content(&mut self, content: &str) -> Result<Vec<i64>> {
        let tx = self.db_connection.transaction()?;
        let current_time = Utc::now().to_rfc3339();

        // Pre-compute line data in single pass
        let lines: Vec<&str> = content.lines().collect();
        let line_data: Vec<(String, String)> = lines.iter()
            .map(|line| {
                let trimmed = line.trim();
                (trimmed.to_string(), generate_content_hash(trimmed))
            })
            .collect();

        // Single query to get existing data
        let mut existing_by_hash: HashMap<String, Vec<i64>> = HashMap::new();
        let mut existing_by_pos: HashMap<i64, (i64, String)> = HashMap::new();
        
        {
            let mut stmt = tx.prepare(
                "SELECT id, content_hash, last_known_line_number FROM lines 
                 WHERE file_path = ? AND is_active = 1"
            )?;
            let rows = stmt.query_map(params![&self.file_path], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?
                ))
            })?;
            
            for row in rows {
                let (id, hash, pos) = row?;
                existing_by_hash.entry(hash.clone()).or_default().push(id);
                if let Some(p) = pos {
                    existing_by_pos.insert(p, (id, hash));
                }
            }
        }

        // Assign IDs efficiently
        let mut used_ids: HashSet<i64> = HashSet::new();
        let mut result_ids = Vec::with_capacity(lines.len());
        let mut batch_updates: Vec<(i64, String, i64)> = Vec::new();
        let mut batch_inserts: Vec<(String, String, i64)> = Vec::new();
        
        for (idx, (content, hash)) in line_data.iter().enumerate() {
            let line_num = (idx + 1) as i64;
            
            let id = if let Some((existing_id, existing_hash)) = existing_by_pos.get(&line_num) {
                if existing_hash == hash && !used_ids.contains(existing_id) {
                    // Perfect match at same position
                    batch_updates.push((*existing_id, content.clone(), line_num));
                    used_ids.insert(*existing_id);
                    *existing_id
                } else {
                    // Position exists but content changed - find reusable ID
                    Self::find_reusable_id(hash, &existing_by_hash, &mut used_ids, 
                                           &mut batch_updates, &mut batch_inserts, 
                                           content, line_num)
                }
            } else {
                // New position
                Self::find_reusable_id(hash, &existing_by_hash, &mut used_ids,
                                       &mut batch_updates, &mut batch_inserts,
                                       content, line_num)
            };
            
            result_ids.push(id);
        }

        // Execute batch operations
        if !batch_updates.is_empty() {
            let mut stmt = tx.prepare(
                "UPDATE lines SET current_content = ?, last_known_line_number = ?, updated_at = ? 
                 WHERE id = ?"
            )?;
            for (id, content, pos) in batch_updates {
                stmt.execute(params![content, pos, current_time, id])?;
            }
        }

        // Handle inserts (need individual execution for ID retrieval)
        if !batch_inserts.is_empty() {
            let mut stmt = tx.prepare(
                "INSERT INTO lines (file_path, current_content, content_hash, is_active, 
                 last_known_line_number, created_at, updated_at) 
                 VALUES (?, ?, ?, 1, ?, ?, ?)"
            )?;
            
            let mut insert_idx = 0;
            for i in 0..result_ids.len() {
                if result_ids[i] == 0 { // Placeholder for new insert
                    let (content, hash, pos) = &batch_inserts[insert_idx];
                    stmt.execute(params![
                        &self.file_path, content, hash, pos, current_time, current_time
                    ])?;
                    result_ids[i] = tx.last_insert_rowid();
                    insert_idx += 1;
                }
            }
        }

        // Batch deactivate unused lines
        let all_existing: HashSet<i64> = existing_by_hash.values().flatten().copied().collect();
        let unused: Vec<i64> = all_existing.difference(&used_ids).copied().collect();
        
        if !unused.is_empty() {
            let placeholders = unused.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "UPDATE lines SET is_active = 0, updated_at = ? WHERE id IN ({})", 
                placeholders
            );
            let mut params_vec = vec![&current_time as &dyn rusqlite::ToSql];
            for id in &unused {
                params_vec.push(id as &dyn rusqlite::ToSql);
            }
            tx.execute(&query, params_vec.as_slice())?;
        }

        tx.commit()?;
        self.line_ids = result_ids.clone();
        Ok(result_ids)
    }

    fn find_reusable_id(
        hash: &str,
        existing_by_hash: &HashMap<String, Vec<i64>>,
        used_ids: &mut HashSet<i64>,
        batch_updates: &mut Vec<(i64, String, i64)>,
        batch_inserts: &mut Vec<(String, String, i64)>,
        content: &str,
        line_num: i64,
    ) -> i64 {
        if let Some(ids) = existing_by_hash.get(hash) {
            for &id in ids {
                if !used_ids.contains(&id) {
                    batch_updates.push((id, content.to_string(), line_num));
                    used_ids.insert(id);
                    return id;
                }
            }
        }
        
        // No reusable ID found - mark for insert
        batch_inserts.push((content.to_string(), hash.to_string(), line_num));
        0 // Placeholder - will be replaced with actual ID after insert
    }

    pub fn get_line_id(&self, line_number: usize) -> Option<i64> {
        if line_number == 0 || line_number > self.line_ids.len() {
            None
        } else {
            Some(self.line_ids[line_number - 1])
        }
    }
}

fn generate_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_tracking_basic() -> Result<()> {
        let test_content = "Line 1\nLine 2\nLine 3\n";
        let mut tracker = LineTracker::new_in_memory("test_file.txt")?;
        
        let ids = tracker.process_file_content(test_content)?;
        assert_eq!(ids.len(), 3);
        
        assert_eq!(tracker.get_line_id(1), Some(ids[0]));
        assert_eq!(tracker.get_line_id(2), Some(ids[1]));
        assert_eq!(tracker.get_line_id(3), Some(ids[2]));
        assert_eq!(tracker.get_line_id(4), None);
        
        Ok(())
    }

    #[test]
    fn test_line_tracking_reuse() -> Result<()> {
        let mut tracker = LineTracker::new_in_memory("test_file2.txt")?;
        
        // First version
        let content1 = "Hello\nWorld\n";
        let ids1 = tracker.process_file_content(content1)?;
        
        // Second version - reorder lines
        let content2 = "World\nHello\n";
        let ids2 = tracker.process_file_content(content2)?;
        
        // IDs should be reused but in different order
        assert_eq!(ids2[0], ids1[1]); // "World" keeps same ID
        assert_eq!(ids2[1], ids1[0]); // "Hello" keeps same ID
        
        Ok(())
    }
}