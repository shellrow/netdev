use std::convert::TryFrom;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InterfaceType {
    Unknown,
    Ethernet,
    TokenRing,
    Fddi,
    BasicIsdn,
    PrimaryIsdn,
    Ppp,
    Loopback,
    Ethernet3Megabit,
    Slip,
    Atm,
    GenericModem,
    FastEthernetT,
    Isdn,
    FastEthernetFx,
    Wireless80211,
    AsymmetricDsl,
    RateAdaptDsl,
    SymmetricDsl,
    VeryHighSpeedDsl,
    IPOverAtm,
    GigabitEthernet,
    Tunnel,
    MultiRateSymmetricDsl,
    HighPerformanceSerialBus,
    Wman,
    Wwanpp,
    Wwanpp2,
}

impl InterfaceType {
    #[cfg(target_os = "windows")]
    pub fn value(&self) -> u32 {
        match *self {
            InterfaceType::Unknown => 1,
            InterfaceType::Ethernet => 6,
            InterfaceType::TokenRing => 9,
            InterfaceType::Fddi => 15,
            InterfaceType::BasicIsdn => 20,
            InterfaceType::PrimaryIsdn => 21,
            InterfaceType::Ppp => 23,
            InterfaceType::Loopback => 24,
            InterfaceType::Ethernet3Megabit => 26,
            InterfaceType::Slip => 28,
            InterfaceType::Atm => 37,
            InterfaceType::GenericModem => 48,
            InterfaceType::FastEthernetT => 62,
            InterfaceType::Isdn => 63,
            InterfaceType::FastEthernetFx => 69,
            InterfaceType::Wireless80211 => 71,
            InterfaceType::AsymmetricDsl => 94,
            InterfaceType::RateAdaptDsl => 95,
            InterfaceType::SymmetricDsl => 96,
            InterfaceType::VeryHighSpeedDsl => 97,
            InterfaceType::IPOverAtm => 114,
            InterfaceType::GigabitEthernet => 117,
            InterfaceType::Tunnel => 131,
            InterfaceType::MultiRateSymmetricDsl => 143,
            InterfaceType::HighPerformanceSerialBus => 144,
            InterfaceType::Wman => 237,
            InterfaceType::Wwanpp => 243,
            InterfaceType::Wwanpp2 => 244,
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn value(&self) -> u32 {
        match *self {
            InterfaceType::Ethernet => 1,
            InterfaceType::TokenRing => 4,
            InterfaceType::Fddi => 774,
            InterfaceType::Ppp => 512,
            InterfaceType::Loopback => 772,
            InterfaceType::Ethernet3Megabit => 2,
            InterfaceType::Slip => 256,
            InterfaceType::Atm => 19,
            InterfaceType::Wireless80211 => 801,
            InterfaceType::Tunnel => 768,
            _ => u32::MAX,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "freebsd", target_os = "netbsd", target_os = "ios"))]
    pub fn value(&self) -> u32 {
        // TODO
        match *self {
            _ => 0,
        }
    }

    pub fn name(&self) -> String {
        match *self {
            InterfaceType::Unknown => String::from("Unknown"),
            InterfaceType::Ethernet => String::from("Ethernet"),
            InterfaceType::TokenRing => String::from("Token Ring"),
            InterfaceType::Fddi => String::from("FDDI"),
            InterfaceType::BasicIsdn => String::from("Basic ISDN"),
            InterfaceType::PrimaryIsdn => String::from("Primary ISDN"),
            InterfaceType::Ppp => String::from("PPP"),
            InterfaceType::Loopback => String::from("Loopback"),
            InterfaceType::Ethernet3Megabit => String::from("Ethernet 3 megabit"),
            InterfaceType::Slip => String::from("SLIP"),
            InterfaceType::Atm => String::from("ATM"),
            InterfaceType::GenericModem => String::from("Generic Modem"),
            InterfaceType::FastEthernetT => String::from("Fast Ethernet T"),
            InterfaceType::Isdn => String::from("ISDN"),
            InterfaceType::FastEthernetFx => String::from("Fast Ethernet FX"),
            InterfaceType::Wireless80211 => String::from("Wireless IEEE 802.11"),
            InterfaceType::AsymmetricDsl => String::from("Asymmetric DSL"),
            InterfaceType::RateAdaptDsl => String::from("Rate Adaptive DSL"),
            InterfaceType::SymmetricDsl => String::from("Symmetric DSL"),
            InterfaceType::VeryHighSpeedDsl => String::from("Very High Data Rate DSL"),
            InterfaceType::IPOverAtm => String::from("IP over ATM"),
            InterfaceType::GigabitEthernet => String::from("Gigabit Ethernet"),
            InterfaceType::Tunnel => String::from("Tunnel"),
            InterfaceType::MultiRateSymmetricDsl => String::from("Multi-Rate Symmetric DSL"),
            InterfaceType::HighPerformanceSerialBus => String::from("High Performance Serial Bus"),
            InterfaceType::Wman => String::from("WMAN"),
            InterfaceType::Wwanpp => String::from("WWANPP"),
            InterfaceType::Wwanpp2 => String::from("WWANPP2"),
        }
    }
}

impl TryFrom<u32> for InterfaceType {
    type Error = ();
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == InterfaceType::Unknown.value() => Ok(InterfaceType::Unknown),
            x if x == InterfaceType::Ethernet.value() => Ok(InterfaceType::Ethernet),
            x if x == InterfaceType::TokenRing.value() => Ok(InterfaceType::TokenRing),
            x if x == InterfaceType::Fddi.value() => Ok(InterfaceType::Fddi),
            x if x == InterfaceType::BasicIsdn.value() => Ok(InterfaceType::BasicIsdn),
            x if x == InterfaceType::PrimaryIsdn.value() => Ok(InterfaceType::PrimaryIsdn),
            x if x == InterfaceType::Ppp.value() => Ok(InterfaceType::Ppp),
            x if x == InterfaceType::Loopback.value() => Ok(InterfaceType::Loopback),
            x if x == InterfaceType::Ethernet3Megabit.value() => Ok(InterfaceType::Ethernet3Megabit),
            x if x == InterfaceType::Slip.value() => Ok(InterfaceType::Slip),
            x if x == InterfaceType::Atm.value() => Ok(InterfaceType::Atm),
            x if x == InterfaceType::GenericModem.value() => Ok(InterfaceType::GenericModem),
            x if x == InterfaceType::FastEthernetT.value() => Ok(InterfaceType::FastEthernetT),
            x if x == InterfaceType::Isdn.value() => Ok(InterfaceType::Isdn),
            x if x == InterfaceType::FastEthernetFx.value() => Ok(InterfaceType::FastEthernetFx),
            x if x == InterfaceType::Wireless80211.value() => Ok(InterfaceType::Wireless80211),
            x if x == InterfaceType::AsymmetricDsl.value() => Ok(InterfaceType::AsymmetricDsl),
            x if x == InterfaceType::RateAdaptDsl.value() => Ok(InterfaceType::RateAdaptDsl),
            x if x == InterfaceType::SymmetricDsl.value() => Ok(InterfaceType::SymmetricDsl),
            x if x == InterfaceType::VeryHighSpeedDsl.value() => Ok(InterfaceType::VeryHighSpeedDsl),
            x if x == InterfaceType::IPOverAtm.value() => Ok(InterfaceType::IPOverAtm),
            x if x == InterfaceType::GigabitEthernet.value() => Ok(InterfaceType::GigabitEthernet),
            x if x == InterfaceType::Tunnel.value() => Ok(InterfaceType::Tunnel),
            x if x == InterfaceType::MultiRateSymmetricDsl.value() => Ok(InterfaceType::MultiRateSymmetricDsl),
            x if x == InterfaceType::HighPerformanceSerialBus.value() => Ok(InterfaceType::HighPerformanceSerialBus),
            x if x == InterfaceType::Wman.value() => Ok(InterfaceType::Wman),
            x if x == InterfaceType::Wwanpp.value() => Ok(InterfaceType::Wwanpp),
            x if x == InterfaceType::Wwanpp2.value() => Ok(InterfaceType::Wwanpp2),
            _ => Err(()),
        }
    }
}
