use super::{AdapterKey, PresentModeChoice, adapter_ids, parse_present_mode, pick_present_mode};
use wgpu::{DeviceType, PresentMode};

fn key<'a>(
    pci: &'a str,
    device_type: DeviceType,
    driver: &'a str,
    name: &'a str,
) -> AdapterKey<'a> {
    AdapterKey {
        pci_bus_id: pci,
        device_type,
        driver,
        name,
    }
}

/// An id names the device, not its place in the list: the same adapters enumerated in another
/// order keep their ids.
#[test]
fn an_id_does_not_change_with_the_enumeration_order() {
    let amd = key(
        "0000:65:00.0",
        DeviceType::IntegratedGpu,
        "radv",
        "AMD Radeon",
    );
    let nvidia = key(
        "0000:01:00.0",
        DeviceType::DiscreteGpu,
        "NVIDIA",
        "RTX 4060",
    );
    let forward = adapter_ids(&[amd, nvidia]);
    let backward = adapter_ids(&[nvidia, amd]);
    assert_eq!(forward, ["pci-0000:65:00.0", "pci-0000:01:00.0"]);
    assert_eq!(backward, ["pci-0000:01:00.0", "pci-0000:65:00.0"]);
}

/// Two identical cards in two slots stay apart, which a name or vendor:device pair cannot do.
#[test]
fn identical_cards_differ_by_slot() {
    let ids = adapter_ids(&[
        key(
            "0000:01:00.0",
            DeviceType::DiscreteGpu,
            "NVIDIA",
            "RTX 4060",
        ),
        key(
            "0000:02:00.0",
            DeviceType::DiscreteGpu,
            "NVIDIA",
            "RTX 4060",
        ),
    ]);
    assert_eq!(ids, ["pci-0000:01:00.0", "pci-0000:02:00.0"]);
}

/// One card served by two drivers is two adapters: each id carries its driver.
#[test]
fn two_drivers_on_one_card_carry_their_driver() {
    let ids = adapter_ids(&[
        key(
            "0000:03:00.0",
            DeviceType::DiscreteGpu,
            "radv",
            "AMD Radeon",
        ),
        key(
            "0000:03:00.0",
            DeviceType::DiscreteGpu,
            "AMD open-source driver",
            "AMD Radeon",
        ),
    ]);
    assert_eq!(
        ids,
        [
            "pci-0000:03:00.0-radv",
            "pci-0000:03:00.0-amd-open-source-driver"
        ]
    );
}

/// A software adapter has no PCI address and is named by its type and driver.
#[test]
fn a_software_adapter_is_named_by_type_and_driver() {
    let ids = adapter_ids(&[key(
        "",
        DeviceType::Cpu,
        "llvmpipe",
        "llvmpipe (LLVM 19.1.7)",
    )]);
    assert_eq!(ids, ["cpu-llvmpipe"]);
}

/// An explicit mode the surface lacks is refused, never swapped for another.
#[test]
fn an_unsupported_explicit_mode_is_refused() {
    let picked = pick_present_mode(
        PresentModeChoice::Explicit(PresentMode::Mailbox),
        None,
        &[PresentMode::Fifo, PresentMode::Immediate],
    );
    let why = picked.expect_err("mailbox is not supported");
    assert!(
        why.contains("mailbox") && why.contains("fifo, immediate"),
        "{why}"
    );
}

/// Automatic falls back to FIFO when the platform's preference is missing.
#[test]
fn automatic_falls_back_to_fifo() {
    let picked = pick_present_mode(
        PresentModeChoice::Auto,
        Some(PresentMode::Mailbox),
        &[PresentMode::Fifo, PresentMode::Immediate],
    );
    assert_eq!(picked.map(|(mode, _)| mode), Ok(PresentMode::Fifo));
}

/// The spellings the command line accepts.
#[test]
fn present_mode_names_parse() {
    assert_eq!(parse_present_mode("FIFO"), Some(PresentMode::Fifo));
    assert_eq!(
        parse_present_mode("immediate"),
        Some(PresentMode::Immediate)
    );
    assert_eq!(parse_present_mode("vsync"), None);
}
