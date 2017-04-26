//extern crate serde;
//use serde::ser::Serializer;

pub trait OscAddress {
    fn build_address(&self, string: &mut String);
    fn get_address(&self) -> String {
        let mut s = String::new();
        self.build_address(&mut s);
        s
    }
    //fn nested_serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
}

