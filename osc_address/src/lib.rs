#[macro_use]
extern crate serde_derive;
extern crate serde;

use std::time::{UNIX_EPOCH, Duration, SystemTime};

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
    messages: Vec<OscPacket<M>>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum OscPacket<M> {
    Message(M),
    Bundle(OscBundle<M>),
}

#[derive(Copy, Clone)]
pub enum OscTime {
    /// Indication to execute the bundle contents immediately upon receipt.
    Now,
    At(AbsOscTime),
}

#[derive(Copy, Clone)]
pub struct AbsOscTime {
    time: (u32, u32),
}

impl<M> OscBundle<M> {
    pub fn time_tag(&self) -> OscTime {
        OscTime::new(self.time_tag)
    }
    pub fn messages(&self) -> &Vec<OscPacket<M>> {
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
    /// Convert the OSC time tag into a type from the std::time library.
    /// If `self == OscTime::now`, it will return the current system time.
    /// Note that this can fail to unrepresentable times, in which case None
    /// is returned.
    ///
    /// See AbsOscTime::as_system_time for more details.
    pub fn as_system_time(&self) -> Option<SystemTime> {
        match *self {
            OscTime::At(ref abs_time) => abs_time.as_system_time(),
            OscTime::Now => Some(SystemTime::now()),
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
    /// Convert the OSC time tag into a type from the std::time library.
    /// This may fail because std::time only allows times >= unix epoch (1970),
    /// whereas OSC allows times >= 1900.
    /// Upon such failure, 'None' is returned.
    pub fn as_system_time(&self) -> Option<SystemTime> {
        // Converting ntp time to unix time, described here: http://stackoverflow.com/a/29138806/216292
        // There are 70 years between 1900 and 1970; 17 of them are leap years.
        // Leap seconds do not need to be considered, as they were introduced in 1972.
        let delta_1970_1900 = (70*365 + 17)*86400;
        let secs_unix = self.sec().checked_sub(delta_1970_1900);
        secs_unix.map(|secs_unix| {
            // u32 frac to nanos is (frac / 2^32) * 10^9,
            // NOTE: will never overflow; a u32 * 10*9 can always fit inside a u64,
            // and a u64 >> 32 always fits in a u32.
            let nanos = (self.frac() as u64 * 1000000000) >> 32;
            UNIX_EPOCH + Duration::new(secs_unix as u64, nanos as u32)
        })
    }
}
