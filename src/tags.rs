use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::collections::hash_map::{Entry, HashMap};
use std::fmt::{self, Display, Formatter};

use TagsError::*;

pub type TagId = u32;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tags {
    tag_ids: HashMap<String, TagId>,
    tag_names: Vec<String>,
}

impl Tags {
    pub fn new() -> Tags {
        Tags {
            tag_ids: HashMap::new(),
            tag_names: Vec::new(),
        }
    }

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

    pub fn get_id(&self, tag_name: &str) -> Option<TagId> {
        self.tag_ids.get(tag_name).copied()
    }

    pub fn get_name(&self, tag_id: TagId) -> Option<&str> {
        self.tag_names.get(tag_id as usize).map(String::as_ref)
    }

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

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TagsError {
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
