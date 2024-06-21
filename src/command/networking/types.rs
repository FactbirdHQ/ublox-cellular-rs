//! Argument and parameter types used by Networking Commands and Responses
use core::fmt::Write;

use atat::AtatLen;
use serde::{Serialize, Serializer};

/// Port filtering enable/disable
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EmbeddedPortFilteringMode {
    /// 0: disable. The configured range is removed.
    Disable,
    /// 1: enable. The <port_range> parameter is mandatory
    Enable(u16, u16),
}

impl AtatLen for EmbeddedPortFilteringMode {
    const LEN: usize = 20;
}

impl Serialize for EmbeddedPortFilteringMode {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            EmbeddedPortFilteringMode::Disable => Serializer::serialize_u8(serializer, 0),
            EmbeddedPortFilteringMode::Enable(port_start, port_end) => {
                let mut serde_state =
                    atat::serde_at::serde::ser::Serializer::serialize_tuple_variant(
                        serializer,
                        "EmbeddedPortFilteringMode",
                        1 as u32,
                        "Enable",
                        0,
                    )?;
                atat::serde_at::serde::ser::SerializeTupleVariant::serialize_field(
                    &mut serde_state,
                    &{
                        let mut str = heapless::String::<16>::new();
                        str.write_fmt(format_args!("{}-{}", port_start, port_end))
                            .ok();
                        str
                    },
                )?;
                atat::serde_at::serde::ser::SerializeTupleVariant::end(serde_state)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use atat::serde_at::ser::to_slice;

    #[test]
    fn serialize_embedded_port_filtering_mode() {
        let options = atat::serde_at::SerializeOptions {
            value_sep: false,
            ..atat::serde_at::SerializeOptions::default()
        };
        let mut buf = [0u8; 32];
        let s = to_slice(
            &EmbeddedPortFilteringMode::Enable(6000, 6200),
            "",
            &mut buf,
            options,
        )
        .unwrap();

        assert_eq!(&buf[..s], b"1,\"6000-6200\"")
    }
}
