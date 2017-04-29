#[macro_use]
extern crate osc_address_derive;
extern crate osc_address;
extern crate serde_osc;
use osc_address::OscMessage;

#[derive(OscMessage)]
enum MyStruct {
    #[osc_address(address="first")]
    First((), ()),
    Second(i32, ()),
}

#[test]
fn path() {
    let msg = MyStruct::Second(42, ());
    assert_eq!(msg.get_address(), "/42");
}

#[test]
fn serialize() {
    let msg = MyStruct::Second(42, ());
    let serialized = serde_osc::ser::to_vec(&msg).unwrap();
    let expected: Vec<u8> = b"\0\0\0\x08/42\0,\0\0\0".iter().cloned().collect();
    assert_eq!(serialized, expected);
}
