use terminus_protocols::link::{ControlFrame, DataFrame, LinkFrame, LinkFrameChild};
use terminus_protocols::PdlPacket;

#[test]
fn data_frame_roundtrip_via_parent_specialize() {
    let data = DataFrame {
        version: 1,
        src: 1,
        dst: 6,
        seq: 42,
        flow_id: 0,
        payload: b"hello".to_vec(),
    };
    let bytes = data.encode_to_vec().unwrap();
    assert_eq!(bytes[0], 0x48, "magic");
    assert_eq!(bytes[2], 0, "frame_type DATA");
    let frame = LinkFrame::decode_full(&bytes).unwrap();
    assert_eq!((frame.src, frame.dst, frame.seq), (1, 6, 42));
    match frame.specialize().unwrap() {
        LinkFrameChild::DataFrame(d) => assert_eq!(d, data),
        other => panic!("wrong child: {other:?}"),
    }
}

#[test]
fn control_frame_roundtrip() {
    let ctl = ControlFrame { version: 1, src: 3, dst: 6, seq: 7, opcode: 1, args: vec![2, 2] };
    let bytes = ctl.encode_to_vec().unwrap();
    match LinkFrame::decode_full(&bytes).unwrap().specialize().unwrap() {
        LinkFrameChild::ControlFrame(c) => assert_eq!(c, ctl),
        other => panic!("wrong child: {other:?}"),
    }
}

#[test]
fn undecodable_bytes_error_cleanly() {
    // Bad magic → decode error (models COUNT this, never panic).
    assert!(LinkFrame::decode_full(&[0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
    // Truncated → decode error.
    assert!(LinkFrame::decode_full(&[0x48, 0x10]).is_err());
}

#[test]
fn data_payload_offset_is_14() {
    // The dissector glue (dissectors/link_glue.lua) chains the IP
    // dissector at offset 14 for DATA frames; keep them in lock-step.
    let data = DataFrame {
        version: 1,
        src: 1,
        dst: 6,
        seq: 0,
        flow_id: 0,
        payload: vec![0x45, 0xEE],
    };
    let bytes = data.encode_to_vec().unwrap();
    assert_eq!(&bytes[14..], &[0x45, 0xEE]);
}
