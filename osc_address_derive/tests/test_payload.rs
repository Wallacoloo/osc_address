#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate osc_address_derive;
extern crate osc_address;
extern crate serde_osc;
use osc_address::OscMessage;

#[derive(OscMessage)]
enum MsgLeaf  {
    #[osc_address(address="first")]
    First((), MsgData),
    #[osc_address(address="second")]
    Second((), (i32, f32)),
}

#[derive(OscMessage)]
#[derive(Serialize, Deserialize)]
struct MsgData {
    v: i32,
    s: String,
}

#[test]
fn test_payload() {
    let payload = MsgData{ v: 0x01020304, s: "test".to_string() };
    let msg = MsgLeaf::First((), payload);
    let serialized = serde_osc::ser::to_vec(&msg).unwrap();
    let expected: Vec<u8> = b"\0\0\0\x18/first\0\0,is\0\x01\x02\x03\x04test\0\0\0\0".iter().cloned().collect();
    assert_eq!(serialized, expected);
}

