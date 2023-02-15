use std::collections::HashMap;
use std::hash::Hash;

use serde::de::{self, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use serde_yaml::Value;

use crate::schema::{Schema, SchemaList};

/// A struct to hold comma serated string or vec<string> values
#[derive(Debug, Default, Clone)]
pub struct CommaSeperated(pub Vec<String>);

impl<'de> Deserialize<'de> for CommaSeperated {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(CommaSeperated(
            deserializer.deserialize_any(CommaSeperatedVisitor)?,
        ))
    }
}

impl<'se> Serialize for CommaSeperated {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0.is_empty() {
            let seq = serializer.serialize_seq(Some(0))?;
            seq.end()
            // return serializer.serialize_seq(Some(0));
        } else {
            serializer.serialize_str(&self.0.join(", "))
        }
    }
}

struct CommaSeperatedVisitor;
impl<'de> de::Visitor<'de> for CommaSeperatedVisitor {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence or a comma separated list of strings")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(v.split(',').map(|s| s.trim().to_string()).collect())
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();

        while let Some(value) = seq.next_element()? {
            values.push(value);
        }

        Ok(values)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ConfigDatatype {
    Tags(Vec<String>),
    String(String),
    Integer(i32),
    Float(f64),
}

/// example in yaml
/// ``` yaml
/// _schema:
///     SchemaName:
///         fields: id, name, age
///         children: OtherSchemaName
///         filename: "{id}-{name}.yaml"
/// ```
#[derive(Debug, Serialize, Default)]
pub struct SchemaConfig {
    pub items: HashMap<String, SchemaConfigItem>,
}

impl<'de> Deserialize<'de> for SchemaConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SchemaConfigVisitor)
    }
}

struct SchemaConfigVisitor;
impl<'de> Visitor<'de> for SchemaConfigVisitor {
    type Value = SchemaConfig;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut items = HashMap::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value::<SchemaConfigItem>()?;
            items.insert(key, value);
        }
        Ok(SchemaConfig { items })
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SchemaConfigItem {
    #[serde(default)]
    pub fields: CommaSeperated,
    #[serde(default)]
    pub children: CommaSeperated,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// example in yaml, to be change to comma seperated
/// ``` yaml
/// _import:
///    - "{pattern1:?}"
///    - "{?}.{?w}": pattern1, pattern2
///    - ["{?}.{?w}","pat1,pat2"]

#[derive(Debug, Default)]
pub struct ImportConfig {
    pub list: Vec<(String, CommaSeperated)>,
}

impl<'de> Deserialize<'de> for ImportConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ImportConfigVisitor;
        impl<'de> Visitor<'de> for ImportConfigVisitor {
            type Value = (String, CommaSeperated);

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut key = String::new();
                let mut value = CommaSeperated(Vec::new());
                while let Some(k) = map.next_key::<String>()? {
                    key = k;
                    value = map.next_value::<CommaSeperated>()?;
                }
                Ok((key, value))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut key = String::new();
                let mut value = Vec::new();
                if let Some(k) = seq.next_element::<String>()? {
                    key = k;
                }
                while let Some(v) = seq.next_element::<String>()? {
                    value.push(v);
                }
                Ok((key, CommaSeperated(value)))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok((v, CommaSeperated(Vec::new())))
            }
        }

        let value_list = Vec::<Value>::deserialize(deserializer)?;
        let mut list = Vec::new();
        for value in value_list {
            match value.deserialize_any(ImportConfigVisitor) {
                Ok(e) => list.push(e),
                Err(e) => {
                    println!("something went wrong with config, skipped | error: {}", e);
                }
            }
        }
        Ok(ImportConfig { list })
    }
}

impl<'se> Serialize for ImportConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.list.len()))?;
        for (key, value) in self.list.iter() {
            if value.0.is_empty() {
                seq.serialize_element(&key)?;
                continue;
            } else {
                let v = value.0.join(",");
                let map = Value::Mapping(
                    vec![(
                        Value::String(key.to_string()),
                        Value::String(v.to_string()), // Value::String(value.to_string()),
                    )]
                    .into_iter()
                    .collect(),
                );
                seq.serialize_element(&map)?;
            }
            // let map = Value::Mapping(
            //     vec![(
            //         Value::String(key.to_string()),
            //         Value::String(value.to_string()),
            //     )]
            //     .into_iter()
            //     .collect(),
            // );
            // seq.serialize_element(&map)?;
        }
        seq.end()
    }
}

/// meta config
/// ``` yaml
/// _meta:
///     schema: "schema"
///     ignore_schema: true
///     ... schema config
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct MetaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default)]
    pub ignore_schema: bool,
    #[serde(flatten)]
    pub other: SchemaConfigItem,
}

impl MetaConfig {
    fn combine(&mut self, other: &MetaConfig) {
        if self.schema.is_none() {
            self.schema = other.schema.clone();
        }
        // if !other.ignore_schema {
        //     self.ignore_schema = other.ignore_schema;
        // }
        self.other.fields.0.extend(other.other.fields.0.clone());
        self.other.children.0.extend(other.other.children.0.clone());
        // self.other.children.combine(&other.other.children);
        if self.other.filename.is_none() {
            self.other.filename = other.other.filename.clone();
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(rename = "_schema")]
    #[serde(default)]
    pub schema: SchemaConfig,
    #[serde(rename = "_data")]
    #[serde(default)]
    pub data: HashMap<String, ConfigDatatype>,
    #[serde(rename = "_tags")]
    #[serde(default)]
    pub tags: HashMap<String, CommaSeperated>,
    #[serde(rename = "_import")]
    #[serde(default)]
    pub import: ImportConfig,
    #[serde(rename = "_meta")]
    #[serde(default)]
    pub meta: MetaConfig,
    #[serde(flatten)]
    pub uncategorized: Value,
}

impl Config {
    pub fn combine_config(&mut self, other: &Config, higher_priority: bool) {
        // combine schema
        // let sc = self.schema.items;
        for i in other.schema.items.iter() {
            if higher_priority {
                self.schema.items.insert(i.0.to_string(), i.1.clone());
            } else {
                self.schema
                    .items
                    .entry(i.0.to_string())
                    .or_insert(i.1.clone());
            }
        }

        // combine data
        for i in other.data.iter() {
            if higher_priority {
                self.data.insert(i.0.to_string(), i.1.clone());
            } else {
                self.data.entry(i.0.to_string()).or_insert(i.1.clone());
            }
        }

        // combine tags
        for i in other.tags.iter() {
            if higher_priority {
                self.tags.insert(i.0.to_string(), i.1.clone());
            } else {
                self.tags.entry(i.0.to_string()).or_insert(i.1.clone());
            }
        }

        // combine import
        for i in other.import.list.iter() {
            if higher_priority {
                self.import.list.insert(0, i.clone());
                continue;
            } else {
                self.import.list.push(i.clone());
            }
        }

        // combine meta
        // TODO: combine meta
        if higher_priority {
            let mut other_meta = other.meta.clone();
            other_meta.combine(&self.meta);
        } else {
            self.meta.combine(&other.meta);
        }

        // if higher_priority {
        //     self.meta = other.meta.clone();
        // }
        // fuck it will do later
    }
}

#[test]
fn test_combine() {
    let mut config1: Config = serde_yaml::from_str(
        r#"
        _meta:
            type: Anime
        _schema:
            Anime:
                fields: anime_name!, tags(tags), startDate, episodeNum
                children: TaggedVideo
                filename: "{a}-{b}.{ext}"
        _data:
            anime_name: a, b, c
            test1: other
        _tags:
            filename: tags, list
        _import:
            - "format1-1": data
            - "format1-2"
    "#,
    )
    .unwrap();

    let config2: Config = serde_yaml::from_str(
        r#"
        _meta:
            type: Anime
        _schema:
            Anime:
                fields: anime_name2!, tags2(tags), startDate2, episodeNum2
                children: TaggedVideo
                filename: "{a}-{b}.{ext}"
        _data:
            anime_name: a2, b2, c2
            test2: other
        _tags:
            filename: tags2, list2
        _import:
            - "format2-1"
            - "format2-2"
    "#,
    )
    .unwrap();

    dbg!(&config1, &config2);

    config1.combine_config(&config2, true);
    dbg!(&config1);
}

#[test]
fn test_yaml() {
    let config: Config = serde_yaml::from_str(
        r#"
        _meta:
            type: Anime
        _schema:
            Anime:
                fields: anime_name!, tags(tags), startDate, episodeNum
                children: TaggedVideo
                filename: "{a}-{b}.{ext}"
            File:
                fields: ext, filename, tags
                filename: "{filename}.{ext}"
        _data:
            anime_name: a, b, c
            test: other
        _tags:
            filename: tags, list
        _import:
            - "{?}.{ext:?w}": data
            - "{?}.{lala:?}"
            - ["{?}-{?}.{?}", a, b, c]
        other data: a
        "#,
    )
    .unwrap();

    // test to schema
    let schema = SchemaList::from(&config.schema);

    let reserialize = serde_yaml::to_string(&config).unwrap();
    dbg!(config, schema);
    println!("{}", reserialize);
}

#[test]
fn test_meta() {
    let config: Config = serde_yaml::from_str(
        r#"
        _meta:
            schema: "schema"
            ignore_schema: true
        "#,
    )
    .unwrap();

    dbg!(config);

    // test to schema
    // let schema = SchemaList::from(&config.schema);

    // let reserialize = serde_yaml::to_string(&config).unwrap();
    // dbg!(config, schema);
    // println!("{}", reserialize);
}
