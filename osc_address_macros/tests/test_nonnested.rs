#[macro_use]
extern crate osc_address_macros;
extern crate osc_address;
extern crate serde_osc;
use osc_address::OscAddress;

#[derive(OscAddress)]
#[derive(Debug, PartialEq)]
enum MyStruct {
    #[osc_address(address="first")]
    First((), ()),
    #[osc_address(address="second")]
    Second((), (i32, f32)),
}

#[test]
fn path() {
    let msg = MyStruct::Second((), (0i32, 1f32));
    assert_eq!(msg.get_address(), "/second");
}

#[test]
fn serialize() {
    let msg = MyStruct::Second((), (1i32, 0f32));
    let serialized = serde_osc::ser::to_vec(&msg).unwrap();
    let expected: Vec<u8> = b"\0\0\0\x14/second\0,if\0\x00\x00\x00\x01\x00\x00\x00\x00".iter().cloned().collect();
    assert_eq!(serialized, expected);
}
#[test]
fn deserialize() {
    let from = b"\0\0\0\x14/second\0,if\0\x00\x00\x00\x01\x00\x00\x00\x00";
    let parsed: MyStruct = serde_osc::from_slice(from).unwrap();
    let expected = MyStruct::Second((), (1i32, 0f32));
    assert_eq!(parsed, expected);
}
