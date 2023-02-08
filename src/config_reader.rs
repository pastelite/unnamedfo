use std::collections::HashMap;
use std::hash::Hash;

use serde::de::{self, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use serde_yaml::Value;

/// A struct to hold comma serated string or vec<string> values
#[derive(Debug, Default)]
struct CommaSeperated(Vec<String>);

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ConfigDatatype {
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
struct SchemaConfig {
    items: HashMap<String, SchemaConfigItem>,
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

#[derive(Debug, Serialize, Deserialize, Default)]
struct SchemaConfigItem {
    #[serde(default)]
    fields: CommaSeperated,
    #[serde(default)]
    children: CommaSeperated,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
}

/// example in yaml, to be change to comma seperated
/// ``` yaml
/// _import:
///    - "{pattern1:?}"
///    - "{?}.{?w}": pattern1, pattern2
///    - ["{?}.{?w}","pat1,pat2"]

#[derive(Debug, Default)]
struct ImportConfig {
    list: Vec<(String, String)>,
}

impl<'de> Deserialize<'de> for ImportConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values: Vec<Value> = <Vec<Value>>::deserialize(deserializer)?;
        let mut list = Vec::new();
        dbg!(&values);
        for i in values {
            match i {
                Value::String(s) => {
                    list.append(&mut vec![(s, "".to_string())]);
                    // println!("str-{:?}", s)
                }
                Value::Mapping(m) => {
                    let data = m.iter().next().unwrap();
                    list.append(&mut vec![(
                        data.0.as_str().unwrap_or("").to_owned(),
                        data.1.as_str().unwrap_or("").to_owned(),
                    )]);
                    // list.append(other)
                    println!("map-{:?}", m)
                }
                Value::Sequence(s) => {
                    let mut iter = s.iter();
                    let key = iter.next().unwrap();
                    let data = iter.next().unwrap();

                    list.append(&mut vec![(
                        key.as_str().unwrap().to_owned(),
                        data.as_str().unwrap().to_owned(),
                    )]);
                }
                _ => return Err(de::Error::custom("Invalid type")),
            }
            // println!("test-{:?}", i);
        }
        // let mut list = Vec::new();
        // for value in values.iter() {
        //     if (value.)
        // }
        // for value in values.iter() {
        //     println!("{:?}", value);
        // }

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
            if (value.eq("")) {
                seq.serialize_element(&key)?;
                continue;
            } else {
                let map = Value::Mapping(
                    vec![(
                        Value::String(key.to_string()),
                        Value::String(value.to_string()),
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

#[derive(Debug, Serialize, Deserialize, Default)]
struct MetaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<String>,
    #[serde(default)]
    ignore_schema: bool,
    #[serde(flatten)]
    other: SchemaConfigItem,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    #[serde(rename = "_schema")]
    #[serde(default)]
    schema: SchemaConfig,
    #[serde(rename = "_data")]
    #[serde(default)]
    data: HashMap<String, ConfigDatatype>,
    #[serde(rename = "_tags")]
    #[serde(default)]
    tags: HashMap<String, CommaSeperated>,
    #[serde(rename = "_import")]
    #[serde(default)]
    import: ImportConfig,
    #[serde(rename = "_meta")]
    #[serde(default)]
    meta: MetaConfig,
    #[serde(flatten)]
    uncategorized: Value,
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
            - "{?}.{lala:?}":
        other data: a
        "#,
    )
    .unwrap();

    let mut data = HashMap::<String, ConfigDatatype>::new();
    // data.insert(
    //     "test".to_owned(),
    //     ConfigDatatype::Tags(CommaSeperated::new("a,b,c")),
    // );
    // data.insert("number-test".to_owned(), ConfigDatatype::Number(32f64));
    let yaml = serde_yaml::to_string(&data).unwrap();

    let reserialize = serde_yaml::to_string(&config).unwrap();
    dbg!(config, yaml);
    println!("{}", reserialize);
}