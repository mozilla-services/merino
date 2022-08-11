//! Data structures that model information about a requester's device. Device form factor,
//! operating system, and browser are captured.

use std::fmt::{self, Debug};
use std::hash::Hash;

use fake::{Fake, Faker};
use serde::Serialize;

use super::FIREFOX_TEST_VERSIONS;

/// The form factor of the device that sent a given suggestion request.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
pub enum FormFactor {
    /// A desktop computer.
    Desktop,
    /// A mobile device.
    Phone,
    /// A tablet computer.
    Tablet,
    /// Something other than a desktop computer, a mobile device, or a tablet computer.
    Other,
}

impl fmt::Display for FormFactor {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Desktop => write!(fmt, "desktop"),
            Self::Phone => write!(fmt, "phone"),
            Self::Tablet => write!(fmt, "tablet"),
            Self::Other => write!(fmt, "other"),
        }
    }
}

impl<F> fake::Dummy<F> for FormFactor {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &F, rng: &mut R) -> Self {
        match rng.gen_range(0..4) {
            0 => Self::Desktop,
            1 => Self::Phone,
            2 => Self::Tablet,
            _ => Self::Other,
        }
    }
}

/// Simplified Operating System Family
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
pub enum OsFamily {
    /// The Windows operating system.
    Windows,
    /// The macOS operating system.
    MacOs,
    /// The Linux operating system.
    Linux,
    /// The iOS operating system.
    IOs,
    /// The Android operating system.
    Android,
    /// The Chrome OS operating system.
    ChromeOs,
    /// The BlackBerry operating system.
    BlackBerry,
    /// An operating system other than Windows, macOS, Linux, iOS, Android, Chrome OS, or
    /// BlackBerry.
    Other,
}

impl fmt::Display for OsFamily {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows => write!(fmt, "windows"),
            Self::MacOs => write!(fmt, "macos"),
            Self::Linux => write!(fmt, "linux"),
            Self::IOs => write!(fmt, "ios"),
            Self::Android => write!(fmt, "android"),
            Self::ChromeOs => write!(fmt, "chrome os"),
            Self::BlackBerry => write!(fmt, "blackberry"),
            Self::Other => write!(fmt, "other"),
        }
    }
}

impl<F> fake::Dummy<F> for OsFamily {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &F, rng: &mut R) -> Self {
        match rng.gen_range(0..8) {
            0 => Self::Windows,
            1 => Self::MacOs,
            2 => Self::Linux,
            3 => Self::IOs,
            4 => Self::Android,
            5 => Self::ChromeOs,
            6 => Self::BlackBerry,
            _ => Self::Other,
        }
    }
}

/// The web browser used to make a suggestion request.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
pub enum Browser {
    /// The Firefox web browser with the major version number.
    Firefox(u32),
    /// A web browser other than Firefox.
    Other,
}

impl fmt::Display for Browser {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Firefox(version) => write!(fmt, "firefox({})", version),
            Self::Other => write!(fmt, "other"),
        }
    }
}

impl<F> fake::Dummy<F> for Browser {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &F, rng: &mut R) -> Self {
        match rng.gen_range(0..2) {
            0 => Self::Firefox(rng.gen_range(FIREFOX_TEST_VERSIONS)),
            _ => Self::Other,
        }
    }
}

/// The user agent from a suggestion request.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
pub struct DeviceInfo {
    /// The operating system family indicated in the User-Agent header.
    pub os_family: OsFamily,
    /// The device form factor indicated in the User-Agent header.
    pub form_factor: FormFactor,
    /// The web browser indicated in the User-Agent header.
    pub browser: Browser,
}

impl fmt::Display for DeviceInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{}, {}, {}",
            self.os_family, self.form_factor, self.browser,
        )
    }
}

impl<F> fake::Dummy<F> for DeviceInfo {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &F, _rng: &mut R) -> Self {
        DeviceInfo {
            os_family: Faker.fake(),
            form_factor: Faker.fake(),
            browser: Faker.fake(),
        }
    }
}
