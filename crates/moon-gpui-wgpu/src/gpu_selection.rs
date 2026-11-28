//! The GPU and present mode an application asks the renderer for, and the names it asks by.
//!
//! By default the renderer picks both itself: the adapter by the order in `wgpu_context.rs`
//! (display server's GPU, then device type) and the present mode from the platform's preference,
//! FIFO when it has none. An application that lets its user override either — a command-line flag,
//! say — hands the choice over once, before its first window, through [`set_gpu_selection`].
//! An explicit choice is never replaced silently. When it cannot be honoured before the first
//! window exists, the renderer calls the application's `on_unusable` with a message saying why and
//! what is available, and that function does not return — how the launch ends is the
//! application's call. Once a session runs, the same failure (a later window, a recovery after the
//! GPU was lost) is logged and returned as an error instead: the window does not open, the
//! recovery is retried, and the running application is not ended over it.
//!
//! Adapters are named by [`adapter_ids`]: the PCI address the Vulkan driver reports, which stays
//! the same when devices are enumerated in another order and tells two identical cards apart.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Which adapter to render with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AdapterChoice {
    /// The renderer's own order.
    #[default]
    Auto,
    /// The adapter with this id from [`adapter_ids`]; nothing else.
    Id(String),
}

/// Which present mode to configure the window surfaces with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentModeChoice {
    /// The platform's preference, FIFO when it has none.
    #[default]
    Auto,
    /// This mode and no other: a surface that lacks it, or a Mesa driver that
    /// `MESA_VK_WSI_PRESENT_MODE` makes present with another one, refuses the window (see the
    /// module documentation for what a refusal does).
    Explicit(wgpu::PresentMode),
}

/// What an application asks the renderer for. The default leaves both choices to the renderer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GpuSelection {
    /// The adapter to render with.
    pub adapter: AdapterChoice,
    /// The present mode of every window surface.
    pub present_mode: PresentModeChoice,
}

/// The selection with the function that ends the process when it cannot be honoured.
struct Installed {
    selection: GpuSelection,
    on_unusable: fn(String) -> !,
}

static INSTALLED: OnceLock<Installed> = OnceLock::new();

/// Hand the renderer the application's choice of adapter and present mode, before the first
/// window is created. `on_unusable` gets a message when an explicit choice cannot be honoured
/// while the first window is built — an unknown id, an adapter that cannot draw to the window, a
/// mode the surface lacks or an override replaces — and must not return. Later failures do not
/// reach it: they are logged and returned as errors. Only the first call counts.
pub fn set_gpu_selection(selection: GpuSelection, on_unusable: fn(String) -> !) {
    if INSTALLED
        .set(Installed {
            selection,
            on_unusable,
        })
        .is_err()
    {
        log::warn!("GPU selection was already set; keeping the first one");
    }
}

/// The adapter choice in force: the application's, or automatic.
pub(crate) fn adapter_choice() -> AdapterChoice {
    INSTALLED
        .get()
        .map(|installed| installed.selection.adapter.clone())
        .unwrap_or_default()
}

/// The present-mode choice in force: the application's, or automatic.
pub(crate) fn present_mode_choice() -> PresentModeChoice {
    INSTALLED
        .get()
        .map(|installed| installed.selection.present_mode)
        .unwrap_or_default()
}

/// An explicit choice cannot be honoured: hand `message` to the application, which ends the
/// process. Reached only through an explicit choice, which only [`set_gpu_selection`] makes, and
/// only before the first window ([`refuse`]).
fn unusable(message: String) -> ! {
    match INSTALLED.get() {
        Some(installed) => (installed.on_unusable)(message),
        None => panic!("{message}"),
    }
}

/// What tells adapters apart, taken from `wgpu::AdapterInfo` so the naming can be tested without
/// a GPU.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AdapterKey<'a> {
    pub(crate) pci_bus_id: &'a str,
    pub(crate) device_type: wgpu::DeviceType,
    pub(crate) driver: &'a str,
    pub(crate) name: &'a str,
}

impl<'a> AdapterKey<'a> {
    /// The key of an adapter.
    pub(crate) fn of(info: &'a wgpu::AdapterInfo) -> Self {
        Self {
            pci_bus_id: &info.device_pci_bus_id,
            device_type: info.device_type,
            driver: &info.driver,
            name: &info.name,
        }
    }
}

/// A stable id for each adapter, in the order given.
///
/// `pci-<domain:bus:device.function>` for an adapter whose driver reports its PCI address (every
/// hardware Vulkan driver with `VK_EXT_pci_bus_info`): the address names the physical slot, so it
/// does not change when the devices are listed in another order, and two identical cards differ.
/// An adapter without one — a software or virtual device — is `<type>-<driver>`, `llvmpipe` for
/// lavapipe as `cpu-llvmpipe`. When two adapters would share an id, which happens when two drivers
/// serve one card (RADV and AMDVLK, NVK and NVIDIA's), each gets its driver appended; adapters
/// still alike after that are numbered in the order given, the one case where order counts.
pub(crate) fn adapter_ids(keys: &[AdapterKey<'_>]) -> Vec<String> {
    let base: Vec<String> = keys
        .iter()
        .map(|key| {
            if key.pci_bus_id.is_empty() {
                let source = if key.driver.is_empty() {
                    key.name
                } else {
                    key.driver
                };
                format!("{}-{}", device_type_word(key.device_type), slug(source))
            } else {
                format!("pci-{}", key.pci_bus_id.to_ascii_lowercase())
            }
        })
        .collect();
    let with_driver: Vec<String> = base
        .iter()
        .zip(keys)
        .map(|(id, key)| {
            if base.iter().filter(|other| *other == id).count() > 1 && !key.driver.is_empty() {
                format!("{id}-{}", slug(key.driver))
            } else {
                id.clone()
            }
        })
        .collect();
    with_driver
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let alike = with_driver.iter().filter(|other| *other == id).count();
            if alike > 1 {
                let ordinal = with_driver[..index].iter().filter(|o| *o == id).count() + 1;
                format!("{id}-{ordinal}")
            } else {
                id.clone()
            }
        })
        .collect()
}

/// The word an id uses for a device type.
fn device_type_word(device_type: wgpu::DeviceType) -> &'static str {
    match device_type {
        wgpu::DeviceType::DiscreteGpu => "discrete",
        wgpu::DeviceType::IntegratedGpu => "integrated",
        wgpu::DeviceType::VirtualGpu => "virtual",
        wgpu::DeviceType::Cpu => "cpu",
        wgpu::DeviceType::Other => "other",
    }
}

/// Lower-case letters and digits, runs of anything else as one `-`.
fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("unnamed");
    }
    out
}

/// One adapter as [`list_gpus`] reports it.
#[derive(Clone, Debug)]
pub struct GpuListing {
    /// The id [`AdapterChoice::Id`] takes.
    pub id: String,
    /// The adapter's name as its driver reports it.
    pub name: String,
    /// Discrete, integrated, virtual, CPU or other.
    pub device_type: wgpu::DeviceType,
    /// The driver's name, `radv` or `NVIDIA` for instance.
    pub driver: String,
    /// The driver's version string.
    pub driver_info: String,
    /// The graphics API the adapter is reached through.
    pub backend: wgpu::Backend,
}

/// Every adapter the renderer could use, with its id, without creating a window or a device.
/// Whether an adapter can draw to a window is only known once one exists.
#[cfg(not(target_family = "wasm"))]
pub fn list_gpus() -> Vec<GpuListing> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: crate::wgpu_context::native_backends(),
        flags: wgpu::InstanceFlags::default(),
        backend_options: wgpu::BackendOptions::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: None,
    });
    let infos: Vec<wgpu::AdapterInfo> =
        gpui::block_on(instance.enumerate_adapters(crate::wgpu_context::native_backends()))
            .iter()
            .map(|adapter| adapter.get_info())
            .collect();
    let keys: Vec<AdapterKey<'_>> = infos.iter().map(AdapterKey::of).collect();
    adapter_ids(&keys)
        .into_iter()
        .zip(&infos)
        .map(|(id, info)| GpuListing {
            id,
            name: info.name.clone(),
            device_type: info.device_type,
            driver: info.driver.clone(),
            driver_info: info.driver_info.clone(),
            backend: info.backend,
        })
        .collect()
}

/// The present modes [`parse_present_mode`] accepts, as they are spelled.
pub const PRESENT_MODE_NAMES: [&str; 3] = ["fifo", "mailbox", "immediate"];

/// The present mode spelled `name` (case-insensitive), one of [`PRESENT_MODE_NAMES`].
pub fn parse_present_mode(name: &str) -> Option<wgpu::PresentMode> {
    match name.trim().to_ascii_lowercase().as_str() {
        "fifo" => Some(wgpu::PresentMode::Fifo),
        "mailbox" => Some(wgpu::PresentMode::Mailbox),
        "immediate" => Some(wgpu::PresentMode::Immediate),
        _ => None,
    }
}

/// How a present mode is spelled in messages.
pub fn present_mode_name(mode: wgpu::PresentMode) -> &'static str {
    match mode {
        wgpu::PresentMode::Fifo => "fifo",
        wgpu::PresentMode::FifoRelaxed => "fifo-relaxed",
        wgpu::PresentMode::Mailbox => "mailbox",
        wgpu::PresentMode::Immediate => "immediate",
        wgpu::PresentMode::AutoVsync => "auto-vsync",
        wgpu::PresentMode::AutoNoVsync => "auto-no-vsync",
    }
}

/// An environment variable set for this process that changes which adapters exist or how frames
/// are presented, behind the application's back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvOverride {
    /// The variable.
    pub name: &'static str,
    /// Its value.
    pub value: String,
    /// What it does.
    pub effect: &'static str,
}

/// The variables [`env_overrides`] looks for, with what each does.
const KNOWN_OVERRIDES: [(&str, &str); 11] = [
    (
        "MESA_VK_WSI_PRESENT_MODE",
        "Mesa drivers present with this mode instead of the one asked for",
    ),
    (
        "MESA_VK_DEVICE_SELECT",
        "Mesa's device-select layer puts this device first",
    ),
    (
        "MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE",
        "Mesa's device-select layer hides every device but the default one",
    ),
    (
        "DRI_PRIME",
        "Mesa's device-select layer puts the chosen GPU first",
    ),
    (
        "VK_ICD_FILENAMES",
        "the Vulkan loader loads only the drivers listed",
    ),
    (
        "VK_DRIVER_FILES",
        "the Vulkan loader loads only the drivers listed",
    ),
    (
        "VK_LOADER_DRIVERS_SELECT",
        "the Vulkan loader loads only the drivers matched",
    ),
    (
        "VK_LOADER_DRIVERS_DISABLE",
        "the Vulkan loader skips the drivers matched",
    ),
    (
        "__NV_PRIME_RENDER_OFFLOAD",
        "NVIDIA's driver offers its GPU for PRIME render offload",
    ),
    (
        "__VK_LAYER_NV_optimus",
        "NVIDIA's Optimus layer reorders or hides Vulkan devices",
    ),
    (
        "ZED_DEVICE_ID",
        "the automatic choice puts the adapter with this PCI device id first",
    ),
];

/// The known variables from [`KNOWN_OVERRIDES`] that are set, non-empty, for this process.
pub fn env_overrides() -> Vec<EnvOverride> {
    KNOWN_OVERRIDES
        .iter()
        .filter_map(|(name, effect)| {
            let value = std::env::var(name).ok()?;
            (!value.trim().is_empty()).then_some(EnvOverride {
                name,
                value,
                effect,
            })
        })
        .collect()
}

/// The present mode to configure, and what decided it, or why an explicit one cannot be had.
///
/// An explicit mode is taken as it is or refused; automatic takes the platform's preference when
/// the surface has it and FIFO, which every surface has, otherwise.
pub(crate) fn pick_present_mode(
    choice: PresentModeChoice,
    preferred: Option<wgpu::PresentMode>,
    supported: &[wgpu::PresentMode],
) -> Result<(wgpu::PresentMode, &'static str), String> {
    match choice {
        PresentModeChoice::Explicit(mode) if supported.contains(&mode) => {
            Ok((mode, "chosen explicitly"))
        }
        PresentModeChoice::Explicit(mode) => Err(format!(
            "present mode {} is not supported here; supported: {}",
            present_mode_name(mode),
            supported
                .iter()
                .map(|mode| present_mode_name(*mode))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        PresentModeChoice::Auto => match preferred.filter(|mode| supported.contains(mode)) {
            Some(mode) => Ok((mode, "automatic, the platform's preference")),
            None => Ok((wgpu::PresentMode::Fifo, "automatic, FIFO")),
        },
    }
}

/// The mode a Mesa driver presents with instead of the one asked for: `MESA_VK_WSI_PRESENT_MODE`
/// when it names a mode Mesa knows and the surface supports — Mesa ignores it otherwise. Matched
/// exactly, as Mesa does (`fifo`, `relaxed`, `mailbox`, `immediate`): `MAILBOX` or `fifo ` are
/// values Mesa rejects, not overrides.
fn mesa_override(
    adapter: &wgpu::AdapterInfo,
    supported: &[wgpu::PresentMode],
) -> Option<(wgpu::PresentMode, String)> {
    if !adapter.driver_info.contains("Mesa") {
        return None;
    }
    let value = std::env::var("MESA_VK_WSI_PRESENT_MODE").ok()?;
    let mode = match value.as_str() {
        "fifo" => wgpu::PresentMode::Fifo,
        "relaxed" => wgpu::PresentMode::FifoRelaxed,
        "mailbox" => wgpu::PresentMode::Mailbox,
        "immediate" => wgpu::PresentMode::Immediate,
        _ => return None,
    };
    supported.contains(&mode).then_some((mode, value))
}

/// Resolve the present mode of a surface on `adapter` and log it with its source.
///
/// An explicit mode the surface lacks, or one a known override would replace, is refused through
/// [`refuse`]: the launch ends before its first window, a later window or a recovery fails with
/// the error. A Mesa driver presents with `MESA_VK_WSI_PRESENT_MODE` instead of the mode asked for;
/// an automatic mode then says so rather than claim the one asked for.
pub(crate) fn resolve_present_mode(
    preferred: Option<wgpu::PresentMode>,
    supported: &[wgpu::PresentMode],
    adapter: &wgpu::AdapterInfo,
) -> anyhow::Result<wgpu::PresentMode> {
    let choice = present_mode_choice();
    let (mode, source) = pick_present_mode(choice, preferred, supported)
        .map_err(|why| refuse(format!("{} ({}): {why}", adapter.name, adapter.driver)))?;
    match mesa_override(adapter, supported).filter(|(applied, _)| *applied != mode) {
        Some((applied, value)) if matches!(choice, PresentModeChoice::Explicit(_)) => {
            return Err(refuse(format!(
                "present mode {} cannot be honoured: MESA_VK_WSI_PRESENT_MODE={value} is set, and \
                 the Mesa driver of {} presents with {} instead; unset it or ask for that mode",
                present_mode_name(mode),
                adapter.name,
                present_mode_name(applied)
            )));
        }
        Some((applied, value)) => log::info!(
            "present mode: {} — MESA_VK_WSI_PRESENT_MODE={value} replaces {} ({source}) in this \
             Mesa driver",
            present_mode_name(applied),
            present_mode_name(mode)
        ),
        None => log::info!("present mode: {} ({source})", present_mode_name(mode)),
    }
    Ok(mode)
}

/// Set once the first window's renderer was built: from then on a running session exists.
static RENDERING_STARTED: AtomicBool = AtomicBool::new(false);

/// A renderer was built: an explicit choice failing from now on must not end the session.
pub(crate) fn mark_rendering_started() {
    RENDERING_STARTED.store(true, Ordering::Relaxed);
}

/// An explicit choice cannot be honoured.
///
/// Before the first window this is the launch the user asked for, and it ends through the
/// application's `on_unusable`. Once a session runs — a later window, a recovery after the GPU was
/// lost, which can fail while the driver comes back from a suspend — it is returned as an error,
/// so that window does not open or the recovery is retried on the next frame, and a running
/// terminal is not killed over it.
pub(crate) fn refuse(message: String) -> anyhow::Error {
    if !RENDERING_STARTED.load(Ordering::Relaxed) {
        unusable(message);
    }
    log::error!("{message}");
    anyhow::anyhow!(message)
}

#[cfg(test)]
mod tests;
