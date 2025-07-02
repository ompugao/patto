use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
//use std::time::Instant;

pub struct LineTracker {
    content_to_id: HashMap<u64, Vec<i64>>,
    position_to_id: HashMap<usize, i64>,
    next_id: i64,
    line_ids: Vec<i64>,
    line_hashes: Vec<u64>,
}

impl LineTracker {
    pub fn new() -> anyhow::Result<Self> {
        Ok(LineTracker {
            content_to_id: HashMap::new(),
            position_to_id: HashMap::new(),
            next_id: 1,
            line_ids: Vec::new(),
            line_hashes: Vec::new(),
        })
    }

    pub fn process_file_content(&mut self, content: &str) -> anyhow::Result<Vec<i64>> {
        let lines: Vec<&str> = content.lines().collect();

        // Fast hash computation using default hasher
        let line_hashes: Vec<u64> = lines
            .iter()
            .map(|line| {
                let mut hasher = DefaultHasher::new();
                line.trim().hash(&mut hasher);
                hasher.finish()
            })
            .collect();

        let mut new_content_to_id: HashMap<u64, Vec<i64>> = HashMap::new();
        let mut new_position_to_id: HashMap<usize, i64> = HashMap::new();
        let mut used_ids: HashSet<i64> = HashSet::new();
        let mut result_ids = Vec::with_capacity(lines.len());

        //let start = Instant::now();
        // Assign IDs with simple in-memory logic
        for (idx, &hash) in line_hashes.iter().enumerate() {
            let line_num = idx + 1;

            let id = if let Some(&existing_id) = self.position_to_id.get(&line_num) {
                // Same position exists
                if let Some(existing_hash) = self.line_hashes.get(idx) {
                    if *existing_hash == hash {
                        // Same content at same position - reuse ID
                        used_ids.insert(existing_id);
                        existing_id
                    } else {
                        // Different content at same position - find reusable ID
                        self.find_or_create_id(hash, &mut used_ids)
                    }
                } else {
                    // Position exists but no hash (shouldn't happen)
                    self.find_or_create_id(hash, &mut used_ids)
                }
            } else {
                // New position
                self.find_or_create_id(hash, &mut used_ids)
            };

            new_content_to_id.entry(hash).or_default().push(id);
            new_position_to_id.insert(line_num, id);
            result_ids.push(id);
        }
        //println!("-- b: {} ms", start.elapsed().as_millis());

        // Update internal state
        self.content_to_id = new_content_to_id;
        self.position_to_id = new_position_to_id;
        self.line_ids = result_ids.clone();
        self.line_hashes = line_hashes;

        Ok(result_ids)
    }

    fn find_or_create_id(&mut self, hash: u64, used_ids: &mut HashSet<i64>) -> i64 {
        // Try to reuse existing ID for this content
        if let Some(existing_ids) = self.content_to_id.get(&hash) {
            for &id in existing_ids {
                if !used_ids.contains(&id) {
                    used_ids.insert(id);
                    return id;
                }
            }
        }

        // No reusable ID found - create new one
        let new_id = self.next_id;
        self.next_id += 1;
        used_ids.insert(new_id);
        new_id
    }

    pub fn get_line_id(&self, line_number: usize) -> Option<i64> {
        if line_number == 0 || line_number > self.line_ids.len() {
            None
        } else {
            Some(self.line_ids[line_number - 1])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_tracking_basic() -> anyhow::Result<()> {
        let test_content = "Line 1\nLine 2\nLine 3\n";
        let mut tracker = LineTracker::new()?;

        let ids = tracker.process_file_content(test_content)?;
        assert_eq!(ids.len(), 3);

        assert_eq!(tracker.get_line_id(1), Some(ids[0]));
        assert_eq!(tracker.get_line_id(2), Some(ids[1]));
        assert_eq!(tracker.get_line_id(3), Some(ids[2]));
        assert_eq!(tracker.get_line_id(4), None);

        Ok(())
    }

    #[test]
    fn test_line_tracking_reuse() -> anyhow::Result<()> {
        let mut tracker = LineTracker::new()?;

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
