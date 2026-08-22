//! Tests against real hardware. Run with `cargo test -p penguinwave-hid -- --ignored`.

use penguinwave_hid::{DeviceRegistry, HidApiBackend};
use std::sync::Arc;

fn registry() -> DeviceRegistry {
    let backend = Arc::new(HidApiBackend::new().expect("hidapi"));
    let mut reg = DeviceRegistry::new(backend);
    reg.refresh().expect("refresh");
    reg
}

#[test]
#[ignore = "needs a HID device"]
fn enumerates_without_error() {
    let reg = registry();
    for d in reg.descriptors() {
        println!(
            "{:04x}:{:04x} {} {:?}",
            d.id.vendor_id, d.id.product_id, d.name, d.presence
        );
    }
}

#[test]
#[ignore = "needs a connected headset"]
fn reads_battery_and_chatmix() {
    let reg = registry();
    let id = reg.selected().expect("no supported headset connected");
    println!("battery: {:?}", reg.battery(id));
    println!("chatmix: {:?}", reg.read_chatmix(id));
}

/// Print the raw status report.
///
/// Byte offsets are the whole protocol for these devices and are not
/// documented anywhere: on a Nova 7 the report reads
/// `[0xb0, _, battery, _, game, chat, ...]`.
#[test]
#[ignore = "needs a connected headset"]
fn dump_raw_status_report() {
    use penguinwave_hid::models::steelseries::DATA_REQUEST;
    use penguinwave_hid::HidApiBackend;
    use penguinwave_hid::HidBackend;
    use penguinwave_proto::DeviceId;

    let backend = HidApiBackend::new().expect("hidapi");
    let mut t = backend
        .open(DeviceId::new(0x1038, 0x2202))
        .expect("open nova 7");
    t.write(&DATA_REQUEST).expect("write");
    let mut buf = [0u8; 64];
    let n = t.read(&mut buf, 1000).expect("read");
    println!("read {n} bytes:");
    for (i, chunk) in buf[..n.max(16)].chunks(8).enumerate() {
        println!("  [{:02}] {:?}", i * 8, chunk);
    }
}
