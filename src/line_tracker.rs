use rusqlite::{Connection, params};
use std::collections::{HashMap, HashSet};
// use std::io::{BufReader, BufRead};
use sha2::{Digest, Sha256};
use chrono::Utc;
use anyhow::Result;
use Drop;

#[derive(Debug, Clone)]
struct TrackedLine {
    id: i64,
    content: String,
    hash: String,
    is_active: bool,
    last_known_line_number: Option<i64>,
}

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
        let current_time_str = Utc::now().to_rfc3339();

        // Get all currently active lines for this file
        let old_active_lines_by_order: HashMap<i64, TrackedLine>;
        let old_active_hashes_to_ids: HashMap<String, Vec<i64>>;
        let old_active_ids: HashSet<i64>;

        {
            let mut stmt = tx.prepare(
                "SELECT id, current_content, content_hash, is_active, last_known_line_number 
                 FROM lines WHERE file_path = ? AND is_active = 1 
                 ORDER BY last_known_line_number ASC"
            )?;
            let active_lines_iter = stmt.query_map(params![&self.file_path], |row| {
                let line = TrackedLine {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    hash: row.get(2)?,
                    is_active: row.get(3)?,
                    last_known_line_number: row.get(4)?,
                };
                Ok(line)
            })?;

            let mut temp_lines_by_order = HashMap::new();
            let mut temp_hashes_to_ids: HashMap<_, Vec<i64>> = HashMap::new();
            let mut temp_ids = HashSet::new();

            for line_result in active_lines_iter {
                let line = line_result?;
                if let Some(order) = line.last_known_line_number {
                    temp_lines_by_order.insert(order, line.clone());
                }
                temp_hashes_to_ids.entry(line.hash.clone()).or_default().push(line.id);
                temp_ids.insert(line.id);
            }
            old_active_lines_by_order = temp_lines_by_order;
            old_active_hashes_to_ids = temp_hashes_to_ids;
            old_active_ids = temp_ids;
        }

        // Process the current file content line by line
        let mut matched_current_run_ids: HashSet<i64> = HashSet::new();
        let mut result_line_ids: Vec<i64> = Vec::new();

        {
            let mut insert_stmt = tx.prepare_cached(
                "INSERT INTO lines (file_path, current_content, content_hash, is_active, last_known_line_number, created_at, updated_at)
                 VALUES (?, ?, ?, 1, ?, ?, ?)"
            )?;
            let mut update_stmt = tx.prepare_cached(
                "UPDATE lines SET current_content = ?, is_active = 1, last_known_line_number = ?, updated_at = ? WHERE id = ?"
            )?;

            for (line_num_0_idx, line) in content.lines().enumerate() {
                let current_line_num = (line_num_0_idx + 1) as i64;
                let trimmed_content = line.trim();
                let content_hash = generate_content_hash(trimmed_content);

                let mut found_match = false;
                let mut line_id: i64 = 0;

                // Check if content is at the same position with same hash
                if let Some(old_line_at_pos) = old_active_lines_by_order.get(&current_line_num) {
                    if old_line_at_pos.hash == content_hash {
                        update_stmt.execute(
                            params![trimmed_content, current_line_num, current_time_str, old_line_at_pos.id]
                        )?;
                        matched_current_run_ids.insert(old_line_at_pos.id);
                        line_id = old_line_at_pos.id;
                        found_match = true;
                    }
                }

                // If not matched at current position, check if this content exists elsewhere
                if !found_match {
                    let mut matched_id_for_reuse: Option<i64> = None;
                    if let Some(ids_for_hash) = old_active_hashes_to_ids.get(&content_hash) {
                        for &id in ids_for_hash {
                            if old_active_ids.contains(&id) && !matched_current_run_ids.contains(&id) {
                                matched_id_for_reuse = Some(id);
                                break;
                            }
                        }
                    }

                    if let Some(id_to_reuse) = matched_id_for_reuse {
                        update_stmt.execute(
                            params![trimmed_content, current_line_num, current_time_str, id_to_reuse]
                        )?;
                        matched_current_run_ids.insert(id_to_reuse);
                        line_id = id_to_reuse;
                    } else {
                        insert_stmt.execute(
                            params![&self.file_path, trimmed_content, content_hash, current_line_num, current_time_str, current_time_str]
                        )?;
                        let new_id = tx.last_insert_rowid();
                        matched_current_run_ids.insert(new_id);
                        line_id = new_id;
                    }
                }

                result_line_ids.push(line_id);
            }

            // Deactivate lines that are no longer in the file
            let mut deactivate_stmt = tx.prepare_cached(
                "UPDATE lines SET is_active = 0, updated_at = ? WHERE id = ?"
            )?;

            for &old_id in &old_active_ids {
                if !matched_current_run_ids.contains(&old_id) {
                    deactivate_stmt.execute(params![current_time_str, old_id])?;
                }
            }
        }

        tx.commit()?;
        self.line_ids = result_line_ids.clone();
        Ok(result_line_ids)
    }

    pub fn get_line_id(&self, line_number: usize) -> Option<i64> {
        if line_number == 0 || line_number > self.line_ids.len() {
            None
        } else {
            Some(self.line_ids[line_number - 1])
        }
    }
}

// impl Drop for LineTracker {
//     fn drop(&mut self) {
//         self.db_connection.close();
//     }
// }

fn generate_content_hash(content: &str) -> String {
    let normalized_line = content.trim();
    let mut hasher = Sha256::new();
    hasher.update(normalized_line.as_bytes());
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
