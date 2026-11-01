use std::ffi::c_void;
use std::mem;

use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
use windows_sys::Win32::System::SystemInformation::{
    IMAGE_FILE_MACHINE, IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_ARM64,
    IMAGE_FILE_MACHINE_I386, OSVERSIONINFOW,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW,
};

use tgow_core::platform::Arch;

pub(crate) fn native_arch() -> Option<Arch> {
    let mut process: IMAGE_FILE_MACHINE = 0;
    let mut native: IMAGE_FILE_MACHINE = 0;
    let current = unsafe { GetCurrentProcess() };
    let succeeded = unsafe { IsWow64Process2(current, &raw mut process, &raw mut native) };
    if succeeded == 0 {
        return None;
    }
    match native {
        IMAGE_FILE_MACHINE_AMD64 => Some(Arch::X86_64),
        IMAGE_FILE_MACHINE_ARM64 => Some(Arch::Aarch64),
        IMAGE_FILE_MACHINE_I386 => Some(Arch::X86),
        _ => None,
    }
}

pub(crate) fn build_number() -> Option<u32> {
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: u32::try_from(mem::size_of::<OSVERSIONINFOW>()).ok()?,
        ..unsafe { mem::zeroed() }
    };
    let status = unsafe { RtlGetVersion(&raw mut info) };
    (status == 0).then_some(info.dwBuildNumber)
}

pub(crate) fn client_animations_enabled() -> bool {
    let mut enabled: i32 = 1;
    let succeeded = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&raw mut enabled).cast::<c_void>(),
            0,
        )
    };
    succeeded == 0 || enabled != 0
}
