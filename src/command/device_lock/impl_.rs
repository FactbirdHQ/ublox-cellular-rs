use super::types::PinStatusCode;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for PinStatusCode {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Self::Ready => Serializer::serialize_bytes(serializer, b"READY"),
            Self::SimPin => Serializer::serialize_bytes(serializer, b"SIM PIN"),
            Self::SimPuk => Serializer::serialize_bytes(serializer, b"SIM PUK"),
            Self::SimPin2 => Serializer::serialize_bytes(serializer, b"SIM PIN2"),
            Self::SimPuk2 => Serializer::serialize_bytes(serializer, b"SIM PUK2"),
            Self::PhNetPin => Serializer::serialize_bytes(serializer, b"PH-NET PIN"),
            Self::PhNetSubPin => Serializer::serialize_bytes(serializer, b"PH-NETSUB PIN"),
            Self::PhSpPin => Serializer::serialize_bytes(serializer, b"PH-SP PIN"),
            Self::PhCorpPin => Serializer::serialize_bytes(serializer, b"PH-CORP PIN"),
            Self::PhSimPin => Serializer::serialize_bytes(serializer, b"PH-SIM PIN"),
        }
    }
}

impl<'de> Deserialize<'de> for PinStatusCode {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        enum Field {
            Ready,
            SimPin,
            SimPuk,
            SimPin2,
            SimPuk2,
            PhNetPin,
            PhNetSubPin,
            PhSpPin,
            PhCorpPin,
            PhSimPin,
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
                    b"READY" => Ok(Field::Ready),
                    b"SIM PIN" => Ok(Field::SimPin),
                    b"SIM PUK" => Ok(Field::SimPuk),
                    b"SIM PIN2" => Ok(Field::SimPin2),
                    b"SIM PUK2" => Ok(Field::SimPuk2),
                    b"PH-NET PIN" => Ok(Field::PhNetPin),
                    b"PH-NETSUB PIN" => Ok(Field::PhNetSubPin),
                    b"PH-SP PIN" => Ok(Field::PhSpPin),
                    b"PH-CORP PIN" => Ok(Field::PhCorpPin),
                    b"PH-SIM PIN" => Ok(Field::PhSimPin),
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
            marker: core::marker::PhantomData<PinStatusCode>,
            lifetime: core::marker::PhantomData<&'de ()>,
        }
        impl<'de> de::Visitor<'de> for Visitor<'de> {
            type Value = PinStatusCode;
            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                core::fmt::Formatter::write_str(formatter, "enum PinStatusCode")
            }

            fn visit_enum<A>(self, data: A) -> core::result::Result<Self::Value, A::Error>
            where
                A: de::EnumAccess<'de>,
            {
                Ok(match de::EnumAccess::variant(data)? {
                    (Field::Ready, _) => PinStatusCode::Ready,
                    (Field::SimPin, _) => PinStatusCode::SimPin,
                    (Field::SimPuk, _) => PinStatusCode::SimPuk,
                    (Field::SimPin2, _) => PinStatusCode::SimPin2,
                    (Field::SimPuk2, _) => PinStatusCode::SimPuk2,
                    (Field::PhNetPin, _) => PinStatusCode::PhNetPin,
                    (Field::PhNetSubPin, _) => PinStatusCode::PhNetSubPin,
                    (Field::PhSpPin, _) => PinStatusCode::PhSpPin,
                    (Field::PhCorpPin, _) => PinStatusCode::PhCorpPin,
                    (Field::PhSimPin, _) => PinStatusCode::PhSimPin,
                })
            }
        }
        const VARIANTS: &[&str] = &[
            "Ready",
            "SimPin",
            "SimPuk",
            "SimPin2",
            "SimPuk2",
            "PhNetPin",
            "PhNetSubPin",
            "PhSpPin",
            "PhCorpPin",
            "PhSimPin",
        ];
        Deserializer::deserialize_enum(
            deserializer,
            "PinStatusCode",
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
    use super::*;
    use crate::command::device_lock::responses::PinStatus;
    use atat::serde_at::de::from_str;
    use atat::serde_at::ser::to_string;
    use heapless::String;

    #[test]
    fn serialize_pin_status() {
        let options = atat::serde_at::SerializeOptions {
            value_sep: false,
            ..atat::serde_at::SerializeOptions::default()
        };
        let s = to_string::<_, 32>(&PinStatusCode::PhNetSubPin, "", options).unwrap();

        assert_eq!(s, String::<32>::from("PH-NETSUB PIN"))
    }

    #[test]
    fn deserialize_pin_status() {
        assert_eq!(
            from_str("+CPIN: READY"),
            Ok(PinStatus {
                code: PinStatusCode::Ready
            })
        );

        assert_eq!(
            from_str("+CPIN: READY\r\n"),
            Ok(PinStatus {
                code: PinStatusCode::Ready
            })
        );

        assert_eq!(
            from_str("+CPIN: SIM PIN\r\n"),
            Ok(PinStatus {
                code: PinStatusCode::SimPin
            })
        );
    }
}
