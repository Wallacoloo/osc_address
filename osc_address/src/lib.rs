pub trait OscAddress {
    fn build_address(&self, string: &mut String);
    fn get_address(&self) -> String;
}

