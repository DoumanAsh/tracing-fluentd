//!Fluentd forward protocol definitions.
use serde::ser::{Serialize, Serializer, SerializeTuple, SerializeMap};

#[derive(Clone)]
#[repr(transparent)]
///HashMap object suitable for fluent record.
pub struct Map(indexmap::IndexMap<String, Value>);

impl Map {
    #[inline(always)]
    ///Creates new empty map.
    pub fn new() -> Self {
        Self(indexmap::IndexMap::new())
    }
}

impl core::fmt::Debug for Map {
    #[inline(always)]
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.0, fmt)
    }
}

impl core::ops::Deref for Map {
    type Target = indexmap::IndexMap<String, Value>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for Map {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

use core::fmt;

#[derive(Debug)]
pub(crate) struct Opts {
    size: usize,
}

#[derive(Clone)]
///Map value type.
pub enum Value {
    ///Boolean
    Bool(bool),
    ///Integer
    Int(i64),
    ///Unsigned integer
    Uint(u64),
    ///String
    Str(String),
    ///Event level
    EventLevel(tracing_core::Level),
    ///Object
    Object(Map),
}

impl From<bool> for Value {
    #[inline(always)]
    fn from(val: bool) -> Self {
        Self::Bool(val)
    }
}

impl From<i64> for Value {
    #[inline(always)]
    fn from(val: i64) -> Self {
        Self::Int(val)
    }
}

impl From<u32> for Value {
    #[inline(always)]
    fn from(val: u32) -> Self {
        Self::Uint(val as _)
    }
}

impl From<u64> for Value {
    #[inline(always)]
    fn from(val: u64) -> Self {
        Self::Uint(val)
    }
}

impl From<String> for Value {
    #[inline(always)]
    fn from(val: String) -> Self {
        Self::Str(val)
    }
}

impl From<tracing::Level> for Value {
    #[inline(always)]
    fn from(val: tracing::Level) -> Self {
        Self::EventLevel(val)
    }
}

impl From<Map> for Value {
    #[inline(always)]
    fn from(val: Map) -> Self {
        Self::Object(val)
    }
}

impl fmt::Debug for Value {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Bool(val) => fmt.write_fmt(format_args!("{}", val)),
            Value::Int(val) => fmt.write_fmt(format_args!("{}", val)),
            Value::Uint(val) => fmt.write_fmt(format_args!("{}", val)),
            Value::EventLevel(val) => fmt.write_fmt(format_args!("{:?}", val)),
            Value::Str(val) => fmt.write_fmt(format_args!("{:?}", val)),
            Value::Object(val) => fmt.write_fmt(format_args!("{:?}", val)),
        }
    }
}

#[derive(Debug)]
///Representation of fluent entry within `Message`
pub struct Record {
    time: u64,
    entries: Map,
}

impl Record {
    ///Creates record with current timestamp
    pub fn now() -> Self {
        let time = match std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH) {
            Ok(time) => time.as_secs(),
            Err(_) => panic!("SystemTime is before UNIX!?"),
        };

        Self {
            time,
            entries: Map::new(),
        }
    }

    ///Merges record entries with provided map
    pub fn update(&mut self, other: &Map) {
        for (key, value) in other.iter() {
            if !self.entries.contains_key(key) {
                self.entries.insert(key.clone(), value.clone());
            }
        }
    }
}

impl core::ops::Deref for Record {
    type Target = Map;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl core::ops::DerefMut for Record {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

#[derive(Debug)]
///Forward mode message.
pub struct Message {
    tag: &'static str,
    entries: Vec<Record>,
    opts: Opts,
    //option
}

impl Message {
    #[inline(always)]
    ///Creates new message with provided tag.
    pub const fn new(tag: &'static str) -> Self {
        Self {
            tag,
            entries: Vec::new(),
            opts: Opts {
                size: 0,
            }
        }
    }

    #[inline(always)]
    ///Adds record to the message.
    pub fn add(&mut self, record: Record) {
        self.entries.push(record);
        self.opts.size += 1;
    }

    #[inline(always)]
    ///Returns number of records inside message.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline(always)]
    ///Clears records from the message
    pub fn clear(&mut self) {
        self.entries.clear();
        self.opts.size = 0;
    }
}

fn tracing_level_to_str(level: tracing_core::Level) -> &'static str {
    if level == tracing_core::Level::ERROR {
        "ERROR"
    } else if level == tracing_core::Level::WARN {
        "WARN"
    } else if level == tracing_core::Level::INFO {
        "INFO"
    } else if level == tracing_core::Level::DEBUG {
        "DEBUG"
    } else {
        "TRACE"
    }
}

impl Serialize for Value {
    #[inline]
    fn serialize<SER: Serializer>(&self, ser: SER) -> Result<SER::Ok, SER::Error> {
        match self {
            Value::Bool(val) => ser.serialize_bool(*val),
            Value::Int(val) => ser.serialize_i64(*val),
            Value::Uint(val) => ser.serialize_u64(*val),
            Value::EventLevel(val) => ser.serialize_str(tracing_level_to_str(*val)),
            Value::Str(val) => ser.serialize_str(val),
            Value::Object(val) => {
                let mut map = ser.serialize_map(Some(val.len()))?;
                for (key, value) in val.iter() {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            },
        }
    }
}

impl Serialize for Map {
    #[inline]
    fn serialize<SER: Serializer>(&self, ser: SER) -> Result<SER::Ok, SER::Error> {
        let mut map = ser.serialize_map(Some(self.0.len()))?;
        for (key, value) in self.0.iter() {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl Serialize for Opts {
    #[inline]
    fn serialize<SER: Serializer>(&self, ser: SER) -> Result<SER::Ok, SER::Error> {
        let mut map = ser.serialize_map(Some(1))?;
        map.serialize_entry("size", &self.size)?;
        map.end()
    }
}

impl Serialize for Record {
    #[inline]
    fn serialize<SER: Serializer>(&self, ser: SER) -> Result<SER::Ok, SER::Error> {
        let mut seq = ser.serialize_tuple(2)?;
        seq.serialize_element(&self.time)?;
        seq.serialize_element(&self.entries)?;
        seq.end()
    }
}

impl Serialize for Message {
    #[inline]
    fn serialize<SER: Serializer>(&self, ser: SER) -> Result<SER::Ok, SER::Error> {
        let mut seq = ser.serialize_tuple(3)?;
        seq.serialize_element(&self.tag)?;
        seq.serialize_element(&self.entries)?;
        seq.serialize_element(&self.opts)?;
        seq.end()
    }
}
