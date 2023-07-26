use super::urc::PacketSwitchedEventReporting;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for PacketSwitchedEventReporting {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Self::NetworkDetach => Serializer::serialize_bytes(serializer, b"NW DETACH"),
            Self::MobileStationDetach => Serializer::serialize_bytes(serializer, b"ME DETACH"),
            Self::NetworkDeactivate => Serializer::serialize_bytes(serializer, b"NW DEACT"),
            Self::MobileStationDeactivate => Serializer::serialize_bytes(serializer, b"ME DEACT"),
            Self::NetworkPDNDeactivate => Serializer::serialize_bytes(serializer, b"NW PDN DEACT"),
            Self::MobileStationPDNDeactivate => {
                Serializer::serialize_bytes(serializer, b"ME PDN DEACT")
            }
        }
    }
}

impl<'de> Deserialize<'de> for PacketSwitchedEventReporting {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        enum Field {
            NetworkDetach,
            MobileStationDetach,
            NetworkDeactivate,
            MobileStationDeactivate,
            NetworkPDNDeactivate,
            MobileStationPDNDeactivate,
        }
        struct FieldVisitor;

        impl<'de> de::Visitor<'de> for FieldVisitor {
            type Value = Field;
            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                core::fmt::Formatter::write_str(formatter, "variant identifier")
            }

            fn visit_bytes<E>(self, value: &[u8]) -> core::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    b"NW DETACH" => Ok(Field::NetworkDetach),
                    b"ME DETACH" => Ok(Field::MobileStationDetach),
                    b"NW DEACT" => Ok(Field::NetworkDeactivate),
                    b"ME DEACT" => Ok(Field::MobileStationDeactivate),
                    b"NW PDN DEACT" => Ok(Field::NetworkPDNDeactivate),
                    b"ME PDN DEACT" => Ok(Field::MobileStationPDNDeactivate),
                    _ => {
                        let value =
                            core::str::from_utf8(value).unwrap_or("\u{fffd}\u{fffd}\u{fffd}");
                        Err(de::Error::unknown_variant(value, VARIANTS))
                    }
                }
            }
        }

        impl<'de> Deserialize<'de> for Field {
            #[inline]
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserializer::deserialize_identifier(deserializer, FieldVisitor)
            }
        }
        struct Visitor<'de> {
            marker: core::marker::PhantomData<PacketSwitchedEventReporting>,
            lifetime: core::marker::PhantomData<&'de ()>,
        }
        impl<'de> de::Visitor<'de> for Visitor<'de> {
            type Value = PacketSwitchedEventReporting;
            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                core::fmt::Formatter::write_str(formatter, "enum PacketSwitchedEventReporting")
            }

            fn visit_enum<A>(self, data: A) -> core::result::Result<Self::Value, A::Error>
            where
                A: de::EnumAccess<'de>,
            {
                Ok(match de::EnumAccess::variant(data)? {
                    (Field::NetworkDetach, _) => PacketSwitchedEventReporting::NetworkDetach,
                    (Field::MobileStationDetach, _) => {
                        PacketSwitchedEventReporting::MobileStationDetach
                    }
                    (Field::NetworkDeactivate, _) => {
                        PacketSwitchedEventReporting::NetworkDeactivate
                    }
                    (Field::MobileStationDeactivate, _) => {
                        PacketSwitchedEventReporting::MobileStationDeactivate
                    }
                    (Field::NetworkPDNDeactivate, _) => {
                        PacketSwitchedEventReporting::NetworkPDNDeactivate
                    }
                    (Field::MobileStationPDNDeactivate, _) => {
                        PacketSwitchedEventReporting::MobileStationPDNDeactivate
                    }
                })
            }
        }
        const VARIANTS: &[&str] = &[
            "NW DETACH",
            "ME DETACH",
            "NW DEACT",
            "ME DEACT",
            "NW PDN DEACT",
            "ME PDN DEACT",
        ];
        Deserializer::deserialize_enum(
            deserializer,
            "PacketSwitchedEventReporting",
            VARIANTS,
            Visitor {
                marker: core::marker::PhantomData::<Self>,
                lifetime: core::marker::PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod test {
    use crate::command::Urc;

    use super::*;
    use atat::serde_at::de::from_str;
    use atat::serde_at::ser::to_string;
    use atat::AtatUrc;
    use heapless::String;

    #[test]
    fn serialize_me_detach() {
        let options = atat::serde_at::SerializeOptions {
            value_sep: false,
            ..atat::serde_at::SerializeOptions::default()
        };
        let s = to_string::<_, 32>(
            &PacketSwitchedEventReporting::MobileStationDetach,
            "",
            options,
        )
        .unwrap();

        assert_eq!(s, String::<32>::from("ME DETACH"))
    }

    #[test]
    fn deserialize_packet_switched_event() {
        assert_eq!(
            from_str("ME DETACH\r\n"),
            Ok(PacketSwitchedEventReporting::MobileStationDetach)
        );

        assert_eq!(
            from_str("NW PDN DEACT\r\n"),
            Ok(PacketSwitchedEventReporting::NetworkPDNDeactivate)
        );

        assert_eq!(
            from_str("NW DETACH\r\n"),
            Ok(PacketSwitchedEventReporting::NetworkDetach)
        );
    }

    #[test]
    fn deserialize_packet_switched_event_urc() {
        let urc = Urc::parse(b"+CGEV: ME DETACH\r\n").unwrap();
        if !matches!(
            urc,
            Urc::PacketSwitchedEventReporting(PacketSwitchedEventReporting::MobileStationDetach),
        ) {
            panic!()
        }
    }
}
