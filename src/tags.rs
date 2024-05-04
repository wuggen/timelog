//! Interval tags.

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::collections::hash_map::{Entry, HashMap};
use std::fmt::{self, Display, Formatter};

use TagsError::*;

/// A tag ID.
///
/// Tags are identified in most places by a simple numerical identifier, which can be used to look
/// up the text name of the tag for display and serialization purposes.
///
/// Tag IDs are assigned in order of tag creation, starting from 0.
pub type TagId = u32;

/// A record of the interval tags in use by a timelog.
///
/// Tag records are serialized as a simple array of tag names. The index of a name in the array is
/// its ID.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tags {
    tag_ids: HashMap<String, TagId>,
    tag_names: Vec<String>,
}

impl Tags {
    /// Create a new, empty tags record.
    pub fn new() -> Tags {
        Tags {
            tag_ids: HashMap::new(),
            tag_names: Vec::new(),
        }
    }

    /// Insert a new tag with the given name.
    ///
    /// Returns an error if a tag by the same name already exists.
    pub fn insert(&mut self, tag_name: &str) -> Result<TagId, TagsError> {
        match self.tag_ids.entry(tag_name.into()) {
            Entry::Occupied(_) => Err(TagExists),
            Entry::Vacant(ent) => {
                let id = self.tag_names.len() as TagId;
                ent.insert(id);
                self.tag_names.push(tag_name.into());
                Ok(id)
            }
        }
    }

    /// Get the tag ID of the tag with the given name, if it exists.
    pub fn get_id(&self, tag_name: &str) -> Option<TagId> {
        self.tag_ids.get(tag_name).copied()
    }

    /// Get the name associated with the given tag ID, if it exists.
    pub fn get_name(&self, tag_id: TagId) -> Option<&str> {
        self.tag_names.get(tag_id as usize).map(String::as_ref)
    }

    /// Insert the tag of the given name if it does not yet exist, and return its tag ID.
    pub fn get_id_or_insert(&mut self, tag_name: &str) -> TagId {
        self.tag_ids
            .get(tag_name)
            .copied()
            .unwrap_or_else(|| self.insert(tag_name).unwrap())
    }
}

impl Serialize for Tags {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.tag_names.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Tags {
    fn deserialize<D>(d: D) -> Result<Tags, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tag_names = Vec::<String>::deserialize(d)?;
        let mut tag_ids = HashMap::new();

        for (id, name) in tag_names.iter().enumerate() {
            match tag_ids.entry(name.into()) {
                Entry::Occupied(_) => return Err(D::Error::custom(TagExists)),
                Entry::Vacant(ent) => {
                    ent.insert(id as TagId);
                }
            }
        }

        Ok(Tags { tag_ids, tag_names })
    }
}

/// Tag record errors.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TagsError {
    /// Attempted to create a tag that already exists.
    TagExists,
}

impl Display for TagsError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TagExists => write!(f, "attempt to insert tag that already exists"),
        }
    }
}

impl std::error::Error for TagsError {}
