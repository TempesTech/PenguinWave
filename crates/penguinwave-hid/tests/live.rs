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
