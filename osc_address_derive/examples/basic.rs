#[macro_use]
extern crate osc_address_derive;
extern crate osc_address;
extern crate serde_osc;

use osc_address::OscMessage;

#[derive(OscMessage)]
pub enum OscToplevel {
    // Any message addressed to /routegraph[/...] will be deserialized into
    // an OscRouteGraph type (defined below).
    #[osc_address(address="routegraph")]
    RouteGraph((), OscRouteGraph),
    // Messages address to /renderer[/...] are deserialized into OscRenderer
    // types.
    #[osc_address(address="renderer")]
    Renderer((), OscRenderer),
}

/// OSC message to /routegraph[/...]
#[derive(OscMessage)]
pub enum OscRouteGraph {
    // Messages to /routegraph/add_node expect one i32 argument.
    #[osc_address(address="add_node")]
    AddNode((), (i32,)),
    // Messages to /routegraph/add_node expect two i32 arguments.
    #[osc_address(address="add_edge")]
    AddEdge((), (i32, i32)),
}

/// OSC message to /renderer[/...]
#[derive(OscMessage)]
pub enum OscRenderer {
    // Messages to /renderer/new expect one i32 argument ...
    #[osc_address(address="new")]
    New((), (i32,)),
    #[osc_address(address="del")]
    Del((), (i32,)),
    // Match messages to /renderer/<u32>[/...]
    ById(u32, OscRendererById),
}

/// OSC message to /renderer/<u32>[/...]
#[derive(OscMessage)]
pub enum OscRendererById {
    // Messages to /renderer/<u32>/say expect one string argument
    #[osc_address(address="say")]
    Say((), (String,)),
}

fn main() {
    // Example of a valid OSC packet that should match the /renderer/<u32>/say address.
    let packet = b"\0\0\0\x28/renderer/42/say\0\0\0\0,s\0\0HELLO, WORLD!\0\0\0";
    // Parse the packet into a OscToplevel enum.
    let message: OscToplevel = serde_osc::from_slice(&packet[..]).unwrap();
    // This should display "id 42 says: HELLO, WORLD!" to the console.
    dispatch(message);
}

/// Dispatch any received OSC message to the appropriate handler.
fn dispatch(msg : OscToplevel) {
    match msg {
        // handle messages to /routegraph[/...]
        OscToplevel::RouteGraph((), rg_msg) => match rg_msg {
            OscRouteGraph::AddNode((), (node_id,)) => println!("Adding a node with id={}", node_id),
            OscRouteGraph::AddEdge((), (n1, n2)) => println!("New edge from {}->{}", n1, n2),
        },
        // handle messages to /renderer[/...]
        OscToplevel::Renderer((), rend_msg) => match rend_msg {
            OscRenderer::ById(renderer_id, by_id_msg) => match by_id_msg {
                // handle messages to /renderer/<renderer_id>/say
                OscRendererById::Say((), (say,)) => println!("id {} says: {}", renderer_id, say),
            },
            // other cases omitted for clarity.
            _ => {},
        }
    }
}
