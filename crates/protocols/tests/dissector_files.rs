//! Guard the checked-in dissector: it must exist (the CLI include_str!s
//! it) and carry both the generated dispatch and the glue registration.

const LUA: &str = include_str!("../dissectors/link.lua");

#[test]
fn dissector_is_generated_with_child_dispatch() {
    assert!(LUA.contains("LinkFrame_protocol"), "generated Proto missing");
    assert!(
        LUA.contains("DataFrame_match_constraints"),
        "child dispatch missing — parent packet must use _body_, then rerun tools/regen-dissector.sh"
    );
}

#[test]
fn dissector_registers_on_user0_and_chains_ip() {
    assert!(LUA.contains(r#"DissectorTable.get("wtap_encap"):add(wtap.USER0"#));
    assert!(LUA.contains(r#"Dissector.get("ip")"#));
}
