// state_wirter provides batch feature for StateDB. The data written to the writer will not be stored to the physical storage unless "commit" using StateDB.
use std::cmp;
use std::collections::HashMap;
use std::sync::Arc;

use neon::prelude::*;
use thiserror::Error;

use crate::batch;
use crate::database::options::IterationOption;
use crate::database::traits::{DatabaseKind, JsNewWithArcMutex, NewDBWithKeyLength};
use crate::database::types::{JsArcMutex, Kind as DBKind};
use crate::diff;
use crate::types::{Cache, KVPair, KeyLength, SharedKVPair, VecOption};
use crate::utils;

pub type SendableStateWriter = JsArcMutex<StateWriter>;

trait Batch {
    fn put(&mut self, key: Box<[u8]>, value: Box<[u8]>);
    fn delete(&mut self, key: Box<[u8]>);
}

#[derive(Error, Debug)]
pub enum StateWriterError {
    #[error("Invalid usage")]
    InvalidUsage,
}

#[derive(Clone, Debug)]
pub struct StateCache {
    init: VecOption,
    value: Vec<u8>,
    dirty: bool,
    deleted: bool,
}

/// StateWriter holds batch of operation for state_db.
#[derive(Default)]
pub struct StateWriter {
    counter: u32,
    pub backup: HashMap<u32, HashMap<Vec<u8>, StateCache>>,
    pub cache: HashMap<Vec<u8>, StateCache>,
}

impl DatabaseKind for StateWriter {
    fn db_kind() -> DBKind {
        DBKind::StateWriter
    }
}

impl Clone for StateWriter {
    fn clone(&self) -> Self {
        let mut cloned = StateWriter::default();
        cloned.cache.clone_from(&self.cache);
        cloned
    }
}

impl NewDBWithKeyLength for StateWriter {
    fn new_db_with_key_length(_: Option<KeyLength>) -> Self {
        Self::default()
    }
}

impl JsNewWithArcMutex for StateWriter {}
impl Finalize for StateWriter {}

impl StateCache {
    fn new(val: &[u8]) -> Self {
        Self {
            init: None,
            value: val.to_vec(),
            dirty: false,
            deleted: false,
        }
    }

    fn new_existing(val: &[u8]) -> Self {
        Self {
            init: Some(val.to_vec()),
            value: val.to_vec(),
            dirty: false,
            deleted: false,
        }
    }
}

impl StateWriter {
    /// cache_new inserts key-value pair as new value.
    pub fn cache_new(&mut self, pair: &SharedKVPair) {
        let cache = StateCache::new(pair.value());
        self.cache.insert(pair.key_as_vec(), cache);
    }

    /// cache_existing inserts key-value pair as updated value.
    pub fn cache_existing(&mut self, pair: &SharedKVPair) {
        let cache = StateCache::new_existing(pair.value());
        self.cache.insert(pair.key_as_vec(), cache);
    }

    /// get returns the value associated with the key.
    /// it returns value, deleted, exists.
    /// - if the value does not exist in the writer it returns ([], false, false).
    /// - if the value exist in the writer but mark as deleted, it returns (val, true, true).
    /// - if the value exists, it returns (val, false, true).
    pub fn get(&self, key: &[u8]) -> (Vec<u8>, bool, bool) {
        let val = self.cache.get(key);
        if val.is_none() {
            return (vec![], false, false);
        }
        let val = val.unwrap();
        if val.deleted {
            return (vec![], true, true);
        }
        (val.value.clone(), false, true)
    }

    /// is_cached returns true if there is value associated with the key.
    /// it is possible key is marked as deleted.
    pub fn is_cached(&self, key: &[u8]) -> bool {
        self.cache.get(key).is_some()
    }

    /// get_range key-value pairs with option specified.
    pub fn get_range(&self, options: &IterationOption) -> Cache {
        let start = options.gte.as_ref().unwrap();
        let end = options.lte.as_ref().unwrap();
        self.cache
            .iter()
            .filter_map(|(k, v)| {
                if utils::compare(k, start) != cmp::Ordering::Less
                    && utils::compare(k, end) != cmp::Ordering::Greater
                    && !v.deleted
                {
                    Some((k.to_vec(), v.value.to_vec()))
                } else {
                    None
                }
            })
            .collect::<Cache>()
    }

    /// update the key with corresponding value.
    pub fn update(&mut self, pair: &KVPair) -> Result<(), StateWriterError> {
        let mut cached = self
            .cache
            .get_mut(pair.key())
            .ok_or(StateWriterError::InvalidUsage)?;
        cached.value = pair.value_as_vec();
        cached.dirty = true;
        cached.deleted = false;
        Ok(())
    }

    /// delete the key in the cache.
    pub fn delete(&mut self, key: &[u8]) {
        let cached = self.cache.get_mut(key);
        if cached.is_none() {
            return;
        }
        let mut cached = cached.unwrap();
        if cached.init.is_none() {
            self.cache.remove(key);
            return;
        }
        cached.deleted = true;
    }

    /// snapshot creates snapshot of the current writer and return the snapshot id.
    fn snapshot(&mut self) -> u32 {
        self.backup.insert(self.counter, self.cache.clone());
        let index = self.counter;
        self.counter += 1;
        index
    }

    /// restore_snapshot reverts the writer to the snapshot id.
    fn restore_snapshot(&mut self, index: u32) -> Result<(), StateWriterError> {
        let backup = self
            .backup
            .get(&index)
            .ok_or(StateWriterError::InvalidUsage)?;
        self.cache.clone_from(backup);
        self.backup = HashMap::new();
        Ok(())
    }

    /// get_updated returns all the updated key-value pairs.
    /// if the key is removed, value will be empty slice.
    pub fn get_updated(&self) -> Cache {
        let mut result = Cache::new();
        for (key, value) in self.cache.iter() {
            if value.init.is_none() || value.dirty {
                result.insert(key.clone(), value.value.clone());
                continue;
            }
            if value.deleted {
                result.insert(key.clone(), vec![]);
            }
        }
        result
    }

    pub fn commit(&self, batch: &mut impl batch::BatchWriter) -> diff::Diff {
        let mut created = vec![];
        let mut updated = vec![];
        let mut deleted = vec![];
        for (key, value) in self.cache.iter() {
            let kv = KVPair::new(key, &value.value);
            if value.init.is_none() {
                created.push(key.to_vec());
                batch.put(&kv);
                continue;
            }
            if value.deleted {
                deleted.push(KVPair::new(key, &value.value));
                batch.delete(key);
                continue;
            }
            if value.dirty {
                updated.push(KVPair::new(key, value.init.as_ref().unwrap()));
                batch.put(&kv);
                continue;
            }
        }
        diff::Diff::new(created, updated, deleted)
    }
}

impl StateWriter {
    /// js_snapshot is handler for JS ffi.
    /// js "this" - StateWriter.
    /// - @returns - snapshot id
    pub fn js_snapshot(mut ctx: FunctionContext) -> JsResult<JsNumber> {
        let writer = ctx
            .this()
            .downcast_or_throw::<SendableStateWriter, _>(&mut ctx)?;

        let batch = Arc::clone(&writer.borrow());
        let mut inner_writer = batch.lock().unwrap();

        let index = inner_writer.snapshot();

        Ok(ctx.number(index))
    }

    /// js_restore_snapshot is handler for JS ffi.
    /// js "this" - StateWriter.
    /// - @params(0) - snapshot id
    pub fn js_restore_snapshot(mut ctx: FunctionContext) -> JsResult<JsUndefined> {
        let writer = ctx
            .this()
            .downcast_or_throw::<SendableStateWriter, _>(&mut ctx)?;

        let batch = Arc::clone(&writer.borrow());
        let mut inner_writer = batch.lock().unwrap();
        let index = ctx.argument::<JsNumber>(0)?.value(&mut ctx) as u32;

        match inner_writer.restore_snapshot(index) {
            Ok(()) => Ok(ctx.undefined()),
            Err(error) => ctx.throw_error(error.to_string())?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::Prefix;

    #[test]
    fn test_cache() {
        let mut writer = StateWriter::default();

        writer.cache_new(&SharedKVPair::new(&[0, 0, 2], &[1, 2, 3]));
        writer.cache_existing(&SharedKVPair::new(&[0, 0, 3], &[1, 2, 4]));

        let (value, deleted, exists) = writer.get(&[0, 0, 2]);
        assert_eq!(value, &[1, 2, 3]);
        assert!(!deleted);
        assert!(exists);

        let (value, deleted, exists) = writer.get(&[0, 0, 3]);
        assert_eq!(value, &[1, 2, 4]);
        assert!(!deleted);
        assert!(exists);

        let (value, deleted, exists) = writer.get(&[0, 0, 1]);
        assert_eq!(value, &[]);
        assert!(!deleted);
        assert!(!exists)
    }

    #[test]
    fn test_state_writer_clone() {
        let mut writer = StateWriter::default();
        writer.cache_new(&SharedKVPair::new(&[1, 2, 3, 4], &[5, 6, 7, 8]));
        writer.cache_new(&SharedKVPair::new(&[10, 20, 30, 40], &[50, 60, 70, 80]));

        let cloned = writer.clone();

        let (value, deleted, exists) = cloned.get(&[1, 2, 3, 4]);
        assert_eq!(value, &[5, 6, 7, 8]);
        assert!(!deleted);
        assert!(exists);

        let (value, deleted, exists) = cloned.get(&[10, 20, 30, 40]);
        assert_eq!(value, &[50, 60, 70, 80]);
        assert!(!deleted);
        assert!(exists);
    }

    #[test]
    fn test_state_writer_cache_new() {
        let mut writer = StateWriter::default();
        assert_eq!(writer.cache.len(), 0);
        writer.cache_new(&SharedKVPair::new(&[1, 2, 3, 4], &[5, 6, 7, 8]));
        assert_eq!(writer.cache.len(), 1);
        writer.cache_new(&SharedKVPair::new(&[10, 20, 30, 40], &[50, 60, 70, 80]));
        assert_eq!(writer.cache.len(), 2);
    }

    #[test]
    fn test_state_writer_cache_existing() {
        let mut writer = StateWriter::default();
        assert_eq!(writer.cache.len(), 0);
        writer.cache_existing(&SharedKVPair::new(&[1, 2, 3, 4], &[5, 6, 7, 8]));
        assert_eq!(writer.cache.len(), 1);
        writer.cache_existing(&SharedKVPair::new(&[10, 20, 30, 40], &[50, 60, 70, 80]));
        assert_eq!(writer.cache.len(), 2);
    }

    #[test]
    fn test_state_writer_is_cached() {
        let mut writer = StateWriter::default();
        assert!(!writer.is_cached(&[1, 2, 3, 4]));

        writer.cache_new(&SharedKVPair::new(&[1, 2, 3, 4], &[5, 6, 7, 8]));
        assert!(writer.is_cached(&[1, 2, 3, 4]));
    }

    #[test]
    fn test_state_writer_get() {
        let mut writer = StateWriter::default();

        let result = writer.get(&[1, 2, 3, 4]);
        assert_eq!(result.0, &[]);
        assert!(!result.1);
        assert!(!result.2);

        writer.cache_existing(&SharedKVPair::new(&[1, 2, 3, 4], &[5, 6, 7, 8]));
        let result = writer.get(&[1, 2, 3, 4]);
        assert_eq!(result.0, &[5, 6, 7, 8]);
        assert!(!result.1);
        assert!(result.2);

        writer.delete(&[1, 2, 3, 4]);
        let result = writer.get(&[1, 2, 3, 4]);
        assert_eq!(result.0, &[]);
        assert!(result.1);
        assert!(result.2);
    }

    #[test]
    fn test_state_writer_update() {
        let mut writer = StateWriter::default();
        writer.cache_new(&SharedKVPair::new(&[1, 2, 3, 4], &[5, 6, 7, 8]));

        writer
            .update(&KVPair::new(&[1, 2, 3, 4], &[9, 10, 11, 12]))
            .unwrap();

        let result = writer.get_updated();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get(&[1, 2, 3, 4].to_vec()).unwrap(),
            &[9, 10, 11, 12]
        );
    }

    #[test]
    fn test_state_writer_delete() {
        let mut writer = StateWriter::default();
        writer.cache_new(&SharedKVPair::new(&[1, 2, 3, 4], &[5, 6, 7, 8]));

        writer.delete(&[1, 2, 3, 4]);
        let result = writer.get(&[1, 2, 3, 4]);
        assert_eq!(result.0, &[]);
        assert!(!result.1);
        assert!(!result.2);

        let mut writer = StateWriter::default();
        writer.cache_existing(&SharedKVPair::new(&[1, 2, 3, 4], &[5, 6, 7, 8]));

        writer.delete(&[1, 2, 3, 4]);
        let result = writer.get(&[1, 2, 3, 4]);
        assert_eq!(result.0, &[]);
        assert!(result.1);
        assert!(result.2);
    }

    #[test]
    fn test_state_writer_snapshot() {
        let mut writer = StateWriter::default();
        writer.cache_new(&SharedKVPair::new(&[1, 2, 3, 4], &[10, 20, 30, 50]));
        writer.cache_new(&SharedKVPair::new(&[5, 6, 7, 8], &[50, 60, 70, 80]));

        writer.snapshot();
        writer.cache_new(&SharedKVPair::new(&[9, 10, 11, 12], &[90, 100, 110, 120]));
        writer.snapshot();
        writer.cache_new(&SharedKVPair::new(&[13, 14, 15, 16], &[130, 140, 150, 160]));

        assert_eq!(writer.cache.len(), 4);

        writer.restore_snapshot(1).unwrap();
        assert_eq!(writer.cache.len(), 3);
    }

    #[test]
    fn test_state_writer_commit() {
        let mut writer = StateWriter::default();
        writer.cache_new(&SharedKVPair::new(&[1, 2, 3, 4], &[10, 20, 30, 50]));
        writer.cache_existing(&SharedKVPair::new(&[5, 6, 7, 8], &[50, 60, 70, 80]));
        writer.cache_existing(&SharedKVPair::new(&[9, 10, 11, 12], &[90, 100, 110, 120]));

        writer.delete(&[5, 6, 7, 8]);
        writer
            .update(&KVPair::new(&[9, 10, 11, 12], &[130, 140, 150, 160]))
            .unwrap();

        let mut write_batch = batch::PrefixWriteBatch::new();
        write_batch.set_prefix(&Prefix::STATE);
        let diff = writer.commit(&mut write_batch);

        let mut batch = batch::PrefixWriteBatch::new();
        batch.set_prefix(&Prefix::STATE);
        diff.revert_commit(&mut batch);
        assert_eq!(batch.batch.len(), 3);
    }
}