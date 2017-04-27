extern crate osc_address;
#[macro_use]
extern crate osc_address_macros;
use osc_address::OscAddress;

#[derive(OscAddress)]
enum MsgLeaf {
    #[osc_address(address="first")]
    First((), ()),
    #[osc_address(address="second")]
    Second((), (i32, f32)),
}

#[derive(OscAddress)]
enum MsgRoot {
    #[osc_address(address="left")]
    Left((), MsgLeaf),
    #[osc_address(address="right")]
    Right((), MsgLeaf),
}

#[test]
fn nested() {
    let msg = MsgRoot::Left((), MsgLeaf::Second((), (-1i32, 0f32)));
    assert_eq!(msg.get_address(), "/left/second");
}

