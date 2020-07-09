use super::types::*;
use serde::{de, export, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for PinStatusCode {
    fn serialize<S>(&self, serializer: S) -> export::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            PinStatusCode::Ready => Serializer::serialize_bytes(serializer, b"READY"),
            PinStatusCode::SimPin => Serializer::serialize_bytes(serializer, b"SIM PIN"),
            PinStatusCode::SimPuk => Serializer::serialize_bytes(serializer, b"SIM PUK"),
            PinStatusCode::SimPin2 => Serializer::serialize_bytes(serializer, b"SIM PIN2"),
            PinStatusCode::SimPuk2 => Serializer::serialize_bytes(serializer, b"SIM PUK2"),
            PinStatusCode::PhNetPin => Serializer::serialize_bytes(serializer, b"PH-NET PIN"),
            PinStatusCode::PhNetSubPin => Serializer::serialize_bytes(serializer, b"PH-NETSUB PIN"),
            PinStatusCode::PhSpPin => Serializer::serialize_bytes(serializer, b"PH-SP PIN"),
            PinStatusCode::PhCorpPin => Serializer::serialize_bytes(serializer, b"PH-CORP PIN"),
            PinStatusCode::PhSimPin => Serializer::serialize_bytes(serializer, b"PH-SIM PIN"),
        }
    }
}

impl<'de> Deserialize<'de> for PinStatusCode {
    fn deserialize<D>(deserializer: D) -> export::Result<Self, D::Error>
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
            fn expecting(&self, formatter: &mut export::Formatter) -> export::fmt::Result {
                export::Formatter::write_str(formatter, "variant identifier")
            }

            fn visit_bytes<E>(self, value: &[u8]) -> export::Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    b"READY" => export::Ok(Field::Ready),
                    b"SIM PIN" => export::Ok(Field::SimPin),
                    b"SIM PUK" => export::Ok(Field::SimPuk),
                    b"SIM PIN2" => export::Ok(Field::SimPin2),
                    b"SIM PUK2" => export::Ok(Field::SimPuk2),
                    b"PH-NET PIN" => export::Ok(Field::PhNetPin),
                    b"PH-NETSUB PIN" => export::Ok(Field::PhNetSubPin),
                    b"PH-SP PIN" => export::Ok(Field::PhSpPin),
                    b"PH-CORP PIN" => export::Ok(Field::PhCorpPin),
                    b"PH-SIM PIN" => export::Ok(Field::PhSimPin),
                    _ => {
                        let value = &export::from_utf8_lossy(value);
                        export::Err(de::Error::unknown_variant(value, VARIANTS))
                    }
                }
            }
        }

        impl<'de> Deserialize<'de> for Field {
            #[inline]
            fn deserialize<D>(deserializer: D) -> export::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserializer::deserialize_identifier(deserializer, FieldVisitor)
            }
        }
        struct Visitor<'de> {
            marker: export::PhantomData<PinStatusCode>,
            lifetime: export::PhantomData<&'de ()>,
        }
        impl<'de> de::Visitor<'de> for Visitor<'de> {
            type Value = PinStatusCode;
            fn expecting(&self, formatter: &mut export::Formatter) -> export::fmt::Result {
                export::Formatter::write_str(formatter, "enum PinStatusCode")
            }

            fn visit_enum<A>(self, data: A) -> export::Result<Self::Value, A::Error>
            where
                A: de::EnumAccess<'de>,
            {
                match match de::EnumAccess::variant(data) {
                    export::Ok(val) => val,
                    export::Err(err) => {
                        return export::Err(err);
                    }
                } {
                    (Field::Ready, _) => export::Ok(PinStatusCode::Ready),
                    (Field::SimPin, _) => export::Ok(PinStatusCode::SimPin),
                    (Field::SimPuk, _) => export::Ok(PinStatusCode::SimPuk),
                    (Field::SimPin2, _) => export::Ok(PinStatusCode::SimPin2),
                    (Field::SimPuk2, _) => export::Ok(PinStatusCode::SimPuk2),
                    (Field::PhNetPin, _) => export::Ok(PinStatusCode::PhNetPin),
                    (Field::PhNetSubPin, _) => export::Ok(PinStatusCode::PhNetSubPin),
                    (Field::PhSpPin, _) => export::Ok(PinStatusCode::PhSpPin),
                    (Field::PhCorpPin, _) => export::Ok(PinStatusCode::PhCorpPin),
                    (Field::PhSimPin, _) => export::Ok(PinStatusCode::PhSimPin),
                }
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
                marker: export::PhantomData::<PinStatusCode>,
                lifetime: export::PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::device_lock::responses::PinStatus;
    use heapless::{consts, String};
    use serde_at::de::from_str;
    use serde_at::ser::to_string;

    #[test]
    fn serialize_pin_status() {
        let options = serde_at::SerializeOptions {
            value_sep: false,
            ..serde_at::SerializeOptions::default()
        };
        let s = to_string::<consts::U32, consts::U32, _>(
            &PinStatusCode::PhNetSubPin,
            String::from(""),
            options,
        )
        .unwrap();

        assert_eq!(s, String::<consts::U32>::from("PH-NETSUB PIN"))
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
            from_str("+CPIN: SIM PIN"),
            Ok(PinStatus {
                code: PinStatusCode::SimPin
            })
        );
    }
}
