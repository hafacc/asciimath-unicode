//! What an installed copy of the keyboard looks like to Windows, and that removing it leaves
//! nothing behind
//!
//! These are skipped unless asked for, since they are only true right after the installer has
//! run or has removed the keyboard again: the `keyboard-windows-build` workflow runs `present`
//! and then `gone`, each as a 64-bit and as a 32-bit program.
#![cfg(windows)]

use std::time::{Duration, Instant};

use asciimath_unicode_tip::ids::{CLASS_ID, PROFILE_ID};
use windows::Win32::Foundation::REGDB_E_CLASSNOTREG;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_InputProcessorProfiles, ITfDisplayAttributeProvider, ITfInputProcessorProfileMgr,
    ITfKeyEventSink, ITfTextInputProcessorEx, TF_INPUTPROCESSORPROFILE,
};
use windows_core::{GUID, IUnknown, Interface, Result};

const CLASS: GUID = GUID::from_u128(CLASS_ID);
const PROFILE: GUID = GUID::from_u128(PROFILE_ID);

/// English (United States), which the keyboard is listed under
const LANGUAGE: u16 = 0x0409;

/// Set COM up on the test's thread, which is left that way until the thread ends
fn set_up_com() {
    // SAFETY: nothing is asked of COM here but to be ready on this thread
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .expect("COM can be set up");
}

/// Have COM find the keyboard's class in the registry, load its library and make a text service
fn create() -> Result<IUnknown> {
    // SAFETY: the class id outlives the call, and every COM object is an `IUnknown`
    unsafe { CoCreateInstance(&CLASS, None, CLSCTX_INPROC_SERVER) }
}

/// Whether Windows lists the keyboard among the input methods for its language
fn is_listed() -> bool {
    // SAFETY: the class id names a system class, and the interface asked for is one of its own
    let profiles: ITfInputProcessorProfileMgr =
        unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER) }
            .expect("Windows has a profile manager");
    // SAFETY: the call takes nothing but a language
    let listed = unsafe { profiles.EnumProfiles(LANGUAGE) }.expect("profiles can be listed");
    std::iter::from_fn(|| {
        let mut profile = [TF_INPUTPROCESSORPROFILE::default()];
        let mut fetched = 0;
        // SAFETY: both point to locals that outlive the call, and there is room for the one
        // profile asked for
        let next = unsafe { listed.Next(&mut profile, &raw mut fetched) };
        (next.is_ok() && fetched == 1).then_some(profile)
    })
    .any(|[profile]| profile.clsid == CLASS && profile.guidProfile == PROFILE)
}

/// Whether Windows comes to list the keyboard as `expected` within a few seconds
fn becomes_listed(expected: bool) -> bool {
    // Windows can take a moment after the installer exits to update the list
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if is_listed() == expected {
            return true;
        } else if Instant::now() >= deadline {
            return false;
        } else {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

#[test]
#[ignore = "only true while the keyboard is installed"]
fn present() {
    set_up_com();
    let service = create().expect("the class is registered and its library loads");
    assert!(service.cast::<ITfTextInputProcessorEx>().is_ok());
    assert!(service.cast::<ITfKeyEventSink>().is_ok());
    assert!(service.cast::<ITfDisplayAttributeProvider>().is_ok());
    assert!(becomes_listed(true));
}

#[test]
#[ignore = "only true once the keyboard has been removed"]
fn gone() {
    set_up_com();
    let error = create().expect_err("the class is no longer registered");
    assert_eq!(error.code(), REGDB_E_CLASSNOTREG);
    assert!(becomes_listed(false));
}
