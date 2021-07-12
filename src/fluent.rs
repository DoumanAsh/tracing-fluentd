use serde::ser::{Serialize, Serializer, SerializeTuple, SerializeMap};

pub struct Opts {
    size: usize,
}

pub struct Record {
    time: u64,
    entries: indexmap::IndexMap<String, String>,
}

//Forward mode message.
pub struct Message {
    tag: &'static str,
    entries: Vec<Record>,
    opts: Opts,
    //option
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
