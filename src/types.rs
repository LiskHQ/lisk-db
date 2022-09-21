use std::collections::HashMap;
use std::ops::Add;

use crate::codec;

pub type NestedVec = Vec<Vec<u8>>;
pub type SharedNestedVec<'a> = Vec<&'a [u8]>;
pub type Cache = HashMap<Vec<u8>, Vec<u8>>;
pub type VecOption = Option<Vec<u8>>;

// Strong type of SMT with max value KEY_LENGTH * 8
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub struct Height(pub u16);

// Strong type of structure position in Subtree with max value 2 ^ SUBTREE_SIZE
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub struct StructurePosition(pub u16);

// Strong type of subtree height with values of SubtreeHeightKind
#[derive(Clone, Debug, Copy)]
pub struct SubtreeHeight(pub SubtreeHeightKind);

#[derive(Clone, Debug, Copy)]
pub struct KeyLength(pub u16);

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SubtreeHeightKind {
    Four = 4,
    Eight = 8,
    Sixteen = 16,
}

#[derive(Debug)]
pub struct DatabaseOptions {
    pub readonly: bool,
    pub key_length: KeyLength,
}

#[derive(Clone, Debug)]
pub struct KVPair(pub Vec<u8>, pub Vec<u8>);

#[derive(Clone, Debug)]
pub struct SharedKVPair<'a>(pub &'a [u8], pub &'a [u8]);

pub trait DB {
    fn get(&self, key: &[u8]) -> Result<VecOption, rocksdb::Error>;
    fn set(&mut self, pair: &KVPair) -> Result<(), rocksdb::Error>;
    fn del(&mut self, key: &[u8]) -> Result<(), rocksdb::Error>;
}

pub trait New {
    fn new() -> Self;
}

pub trait KVPairCodec {
    fn decode(val: &[u8]) -> Result<KVPair, codec::CodecError>;
    fn encode(&self) -> Vec<u8>;
}

impl New for Cache {
    #[inline]
    fn new() -> Self {
        HashMap::new()
    }
}

impl New for NestedVec {
    #[inline]
    fn new() -> Self {
        vec![]
    }
}

impl From<&u8> for Height {
    #[inline]
    fn from(value: &u8) -> Height {
        Height(*value as u16)
    }
}

impl From<u8> for Height {
    #[inline]
    fn from(value: u8) -> Height {
        Height(value as u16)
    }
}

impl From<Height> for u8 {
    #[inline]
    fn from(value: Height) -> u8 {
        value.0 as u8
    }
}

impl From<Height> for usize {
    #[inline]
    fn from(value: Height) -> usize {
        value.0 as usize
    }
}

impl From<f64> for Height {
    #[inline]
    fn from(value: f64) -> Height {
        Self(value as u16)
    }
}

impl Add for Height {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl From<KeyLength> for u16 {
    #[inline]
    fn from(value: KeyLength) -> u16 {
        value.0 as u16
    }
}

impl From<usize> for KeyLength {
    #[inline]
    fn from(value: usize) -> KeyLength {
        Self(value as u16)
    }
}

impl From<KeyLength> for usize {
    #[inline]
    fn from(value: KeyLength) -> usize {
        value.0 as usize
    }
}

impl From<f64> for KeyLength {
    #[inline]
    fn from(value: f64) -> KeyLength {
        Self(value as u16)
    }
}

impl Default for SubtreeHeight {
    #[inline]
    fn default() -> Self {
        SubtreeHeight(SubtreeHeightKind::Eight)
    }
}

impl From<SubtreeHeight> for StructurePosition {
    #[inline]
    fn from(value: SubtreeHeight) -> StructurePosition {
        StructurePosition(value.u16())
    }
}

impl From<StructurePosition> for u8 {
    #[inline]
    fn from(value: StructurePosition) -> u8 {
        value.0 as u8
    }
}

impl From<u8> for StructurePosition {
    #[inline]
    fn from(value: u8) -> StructurePosition {
        StructurePosition(value as u16)
    }
}

impl From<StructurePosition> for Height {
    #[inline]
    fn from(value: StructurePosition) -> Height {
        Height(value.0)
    }
}

impl KVPair {
    #[inline]
    pub fn new(key: &[u8], value: &[u8]) -> Self {
        Self(key.to_vec(), value.to_vec())
    }

    #[inline]
    pub fn key(&self) -> &[u8] {
        &self.0
    }

    #[inline]
    pub fn value(&self) -> &[u8] {
        &self.1
    }

    #[inline]
    pub fn key_as_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    #[inline]
    pub fn value_as_vec(&self) -> Vec<u8> {
        self.1.to_vec()
    }

    #[inline]
    pub fn is_empty_value(&self) -> bool {
        self.1.is_empty()
    }
}

impl<'a> SharedKVPair<'a> {
    pub fn new(key: &'a [u8], value: &'a [u8]) -> Self {
        Self(key, value)
    }

    #[allow(dead_code)]
    pub fn key(&self) -> &[u8] {
        self.0
    }

    pub fn value(&self) -> &[u8] {
        self.1
    }

    pub fn key_as_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Height {
    #[inline]
    pub fn is_equal_to(self, value: u16) -> bool {
        self.0 == value
    }

    #[inline]
    pub fn sub(self, value: u16) -> Self {
        Self(self.0 - value)
    }

    #[inline]
    pub fn add(self, value: u8) -> Self {
        Height(self.0 + value as u16)
    }

    #[inline]
    pub fn sub_to_usize(self, value: u8) -> usize {
        (self.0 - value as u16) as usize
    }

    #[inline]
    pub fn div_to_usize(self, value: u16) -> usize {
        (self.0 / value) as usize
    }

    #[inline]
    pub fn mod_to_u8(self, value: u16) -> u8 {
        (self.0 % value) as u8
    }

    #[inline]
    pub fn to_be_bytes(self) -> [u8; 2] {
        self.0.to_be_bytes()
    }

    // Cast to u32 and returns with len(4) for JS API
    #[inline]
    pub fn as_u32_to_be_bytes(self) -> [u8; 4] {
        (self.0 as u32).to_be_bytes()
    }
}

impl KeyLength {
    // Cast to u32 and returns with len(4) for JS API
    #[inline]
    pub fn as_u32_to_be_bytes(self) -> [u8; 4] {
        (self.0 as u32).to_be_bytes()
    }
}

impl SubtreeHeight {
    #[inline]
    pub fn u16(self) -> u16 {
        self.0 as u16
    }

    #[inline]
    pub fn is_four(self) -> bool {
        self.0 == SubtreeHeightKind::Four
    }

    #[inline]
    pub fn sub_to_usize(self, value: u8) -> usize {
        (self.u16() - value as u16) as usize
    }
}

impl StructurePosition {
    #[inline]
    pub fn add(self, value: u16) -> Self {
        StructurePosition(self.0 + value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_values_subtree_height_kind() {
        let test_data = vec![
            (SubtreeHeightKind::Four, 4u16),
            (SubtreeHeightKind::Eight, 8u16),
            (SubtreeHeightKind::Sixteen, 16u16),
        ];
        for (data, result) in test_data {
            assert_eq!(SubtreeHeight(data).u16(), result);
        }
    }
}