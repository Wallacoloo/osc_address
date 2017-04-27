extern crate serde;

pub trait OscAddress : serde::Serialize {
    fn build_address(&self, string: &mut String);
    fn get_address(&self) -> String {
        let mut s = String::new();
        self.build_address(&mut s);
        s
    }
    fn serialize_body<S: serde::ser::SerializeTuple>(&self, serializer: &mut S) -> Result<(), S::Error>;
}

