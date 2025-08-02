use std::fmt;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Operational state of a network interface.
/// 
/// See also:
/// https://www.kernel.org/doc/Documentation/networking/operstates.txt
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OperState {
    /// Interface state is unknown
    Unknown,
    /// Interface is not present
    NotPresent,
    /// Interface is administratively or otherwise down
    Down,
    /// Interface is down because a lower layer is down
    LowerLayerDown,
    /// Interface is in testing state
    Testing,
    /// Interface is dormant
    Dormant,
    /// Interface is operational
    Up,
}

impl OperState {
    /// Return lowercase string representation matching `/sys/class/net/*/operstate`
    pub fn as_str(&self) -> &'static str {
        match self {
            OperState::Unknown => "unknown",
            OperState::NotPresent => "notpresent",
            OperState::Down => "down",
            OperState::LowerLayerDown => "lowerlayerdown",
            OperState::Testing => "testing",
            OperState::Dormant => "dormant",
            OperState::Up => "up",
        }
    }
}

impl fmt::Display for OperState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for OperState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unknown" => Ok(OperState::Unknown),
            "notpresent" => Ok(OperState::NotPresent),
            "down" => Ok(OperState::Down),
            "lowerlayerdown" => Ok(OperState::LowerLayerDown),
            "testing" => Ok(OperState::Testing),
            "dormant" => Ok(OperState::Dormant),
            "up" => Ok(OperState::Up),
            _ => Err(()),
        }
    }
}
