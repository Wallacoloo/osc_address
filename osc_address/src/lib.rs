#![feature(try_from)]

#[macro_use]
extern crate serde_derive;
extern crate serde;

use std::convert::TryInto;
use std::time::{UNIX_EPOCH, Duration, SystemTime};

/// OSC uses ntp time (epoch of 1900), and std::time uses Unix epoch (1970).
/// This constant is used in conversion between the two formats.
/// There are 70 years between 1900 and 1970; 17 of them are leap years.
/// Leap seconds do not need to be considered, as they were introduced in 1972.
const DELTA_1970_1900: u32 = (70*365 + 17)*86400;

pub trait OscMessage<'m> : serde::Serialize + serde::Deserialize<'m> {
    /// Append the address that this message would be sent to into the given string.
    /// This is intended to be used as a builder method called by `get_address`.
    /// Generally, users should not directly call this function.
    fn build_address(&self, string: &mut String);
    /// Determine the address that this message would be sent to, as a String.
    /// If this type is a struct (i.e. it represents just the payload of a message),
    /// then this method returns an empty string.
    /// In all other cases, it will return a string beginning with "/".
    fn get_address(&self) -> String {
        let mut s = String::new();
        self.build_address(&mut s);
        s
    }
    /// Serialize the payload of this message, and not its address.
    /// In the case that the variants of this message are also enumerated OscMessages,
    /// this method will recurse and serialize the final (i.e. leaf) message payload.
    fn serialize_body<S: serde::ser::SerializeTuple>(&self, serializer: &mut S) -> Result<(), S::Error>;
    /// If `seq` represents the payload of an OSC message (i.e. the argument list),
    /// then this will deserialize the address + data into the appropriate enum
    /// variant.
    ///
    /// In the case that Self is a struct and represents the payload of a message
    /// (without any address), then it is expected that address is either "" or "/".
    fn deserialize_body<D: serde::de::SeqAccess<'m>>(address: String, seq: D) -> Result<Self, D::Error>;
}

#[derive(Serialize, Deserialize)]
pub struct OscBundle<M> {
    time_tag: (u32, u32),
    messages: Vec<OscPacket<M>>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
/// An OSC packet represents either a single OSC message, or a bundle with an
/// associated time of zero or more OSC packets.
pub enum OscPacket<M> {
    Message(M),
    Bundle(OscBundle<M>),
}

#[derive(Copy, Clone)]
pub enum OscTime {
    /// Indication to execute the bundle contents immediately upon receipt.
    Now,
    /// Execute the bundle contents at a specified time.
    At(AbsOscTime),
}

#[derive(Copy, Clone)]
/// OSC uses ntp time, i.e. absolute # of seconds since 1970 + a fraction of a second.
pub struct AbsOscTime {
    sec: u32,
    frac: u32,
}

impl<M> OscBundle<M> {
    pub fn time_tag(&self) -> OscTime {
        OscTime::new(self.time_tag.0, self.time_tag.1)
    }
    pub fn messages(&self) -> &Vec<OscPacket<M>> {
        &self.messages
    }
}

impl OscTime {
    /// Create a OSC time from seconds and a fraction of a second.
    /// In the special case that sec = 0 and frac = 1, this is to be interpreted
    /// as "now" or "immediately upon the time of message receipt."
    pub fn new(sec: u32, frac: u32) -> Self {
        match (sec, frac) {
            (0, 1) => OscTime::Now,
            _ => OscTime::At(AbsOscTime::new(sec, frac)),
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
    /// Create a OSC time from seconds and a fraction of a second.
    /// It is assumed that (sec, frac) != (0, 1) -- otherwise the time should
    /// be represented as OscTime::Now.
    pub fn new(sec: u32, frac: u32) -> Self {
        Self{ sec, frac }
    }
    /// std::time::SystemTime -> AbsOscTime.
    /// Note that the OSC time can only represent dates out to year 2036,
    /// but SystemTime allows greater range. Hence, this returns None if the
    /// time is not representable.
    pub fn from_system_time(t: SystemTime) -> Option<Self> {
        let since_epoch = t.duration_since(UNIX_EPOCH);
        // Unwrap errors; transform them to None.
        let since_epoch = match since_epoch {
            Err(_) => return None,
            Ok(dur) => dur,
        };
        let unix_secs: Result<u32, _> = since_epoch.as_secs().try_into();
        // Unwrap errors; transform them to None.
        let unix_secs = match unix_secs {
            Err(_) => return None,
            Ok(secs) => secs,
        };
        let ntp_secs = unix_secs.checked_add(DELTA_1970_1900);
        ntp_secs.map(|ntp_secs| {
            let frac = ((since_epoch.subsec_nanos() as u64) << 32) / 1000000000;
            Self::new(ntp_secs, frac as u32)
        })
    }
    pub fn sec(&self) -> u32 {
        self.sec
    }
    pub fn frac(&self) -> u32 {
        self.frac
    }
    pub fn sec_frac(&self) -> (u32, u32) {
        (self.sec, self.frac)
    }
    /// Convert the OSC time tag into a type from the std::time library.
    /// This may fail because std::time only allows times >= unix epoch (1970),
    /// whereas OSC allows times >= 1900.
    /// Upon such failure, 'None' is returned.
    pub fn as_system_time(&self) -> Option<SystemTime> {
        // Converting ntp time to unix time, described here: http://stackoverflow.com/a/29138806/216292
        let secs_unix = self.sec().checked_sub(DELTA_1970_1900);
        secs_unix.map(|secs_unix| {
            // u32 frac to nanos is (frac / 2^32) * 10^9,
            // NOTE: will never overflow; a u32 * 10*9 can always fit inside a u64,
            // and a u64 >> 32 always fits in a u32.
            let nanos = (self.frac() as u64 * 1000000000) >> 32;
            UNIX_EPOCH + Duration::new(secs_unix as u64, nanos as u32)
        })
    }
}

