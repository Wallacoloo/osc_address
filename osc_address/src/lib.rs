#[macro_use]
extern crate serde_derive;
extern crate serde;

pub trait OscMessage<'m> : serde::Serialize + serde::Deserialize<'m> {
    fn build_address(&self, string: &mut String);
    fn get_address(&self) -> String {
        let mut s = String::new();
        self.build_address(&mut s);
        s
    }
    fn serialize_body<S: serde::ser::SerializeTuple>(&self, serializer: &mut S) -> Result<(), S::Error>;
    fn deserialize_body<D: serde::de::SeqAccess<'m>>(address: String, seq: D) -> Result<Self, D::Error>;
}

#[derive(Serialize, Deserialize)]
pub struct OscBundle<M> {
    time_tag: (u32, u32),
    messages: Vec<M>,
}

pub enum OscPacket<M> {
    Message(M),
    Bundle(OscBundle<M>),
}

pub enum OscTime {
    /// Indication to execute the bundle contents immediately upon receipt.
    Now,
    At(AbsOscTime),
}

pub struct AbsOscTime {
    time: (u32, u32),
}

impl<M> OscBundle<M> {
    pub fn time_tag(&self) -> OscTime {
        OscTime::new(self.time_tag)
    }
    pub fn messages(&self) -> &Vec<M> {
        &self.messages
    }
}

impl OscTime {
    pub fn new(tag: (u32, u32)) -> Self {
        match tag {
            (0, 1) => OscTime::Now,
            other => OscTime::At(AbsOscTime::new(other)),
        }
    }
}

impl AbsOscTime {
    pub fn new(time: (u32, u32)) -> Self {
        Self{ time }
    }
    pub fn sec(&self) -> u32 {
        self.time.0
    }
    pub fn frac(&self) -> u32 {
        self.time.1
    }
    pub fn sec_frac(&self) -> (u32, u32) {
        self.time
    }
}
