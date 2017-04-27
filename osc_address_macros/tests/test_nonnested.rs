extern crate osc_address;
#[macro_use]
extern crate osc_address_macros;
use osc_address::OscAddress;

#[derive(OscAddress)]
enum MyStruct {
    #[osc_address(address="first")]
    First((), ()),
    #[osc_address(address="second")]
    Second((), (i32, f32)),
}

#[test]
fn nonnested() {
    assert_eq!(MyStruct::Second((), (0i32, 1f32)).get_address(), "/second");
}

