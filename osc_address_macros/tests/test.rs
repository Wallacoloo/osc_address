extern crate osc_address;
#[macro_use]
extern crate osc_address_macros;
use osc_address::OscAddress;

#[derive(OscAddress)]
enum MyStruct {
    #[osc_address(address="first")]
    First((), ()),
    #[osc_address(address="second")]
    Second((), ()),
}

#[test]
fn it_works2() {
    assert_eq!(MyStruct::First((), ()).get_address(), "first");
}

