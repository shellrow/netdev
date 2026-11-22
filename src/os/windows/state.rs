use super::interface::Adapters;
use crate::interface::state::OperState;
use std::ffi::CStr;

pub(super) fn from_status(status: i32) -> OperState {
    match status {
        1 => OperState::Up,
        2 => OperState::Down,
        3 => OperState::Testing,
        5 => OperState::Dormant,
        6 => OperState::NotPresent,
        7 => OperState::LowerLayerDown,
        _ => OperState::Unknown,
    }
}

/// Reads an adapter's operational state by its GUID name.
pub fn operstate(if_name: &str) -> OperState {
    let Some(adapters) = Adapters::read(0) else {
        return OperState::Unknown;
    };
    adapters
        .iter()
        .find(|adapter| {
            !adapter.AdapterName.is_null()
                && unsafe { CStr::from_ptr(adapter.AdapterName.cast()) }.to_bytes()
                    == if_name.as_bytes()
        })
        .map_or(OperState::Unknown, |adapter| {
            from_status(adapter.OperStatus)
        })
}
