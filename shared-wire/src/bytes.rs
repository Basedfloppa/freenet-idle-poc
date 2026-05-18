//! `serde` helpers for fixed-size byte arrays. Native bincode handles
//! `[u8; N]` as a tuple-of-bytes, which inflates the encoding; these
//! helpers emit a single `bytes` token instead.

pub mod byte_array_32 {
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S: Serializer>(v: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = [u8; 32];
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("32 bytes")
            }
            fn visit_bytes<E: Error>(self, b: &[u8]) -> Result<Self::Value, E> {
                b.try_into().map_err(|_| E::custom("expected 32 bytes"))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = [0u8; 32];
                for slot in out.iter_mut() {
                    *slot = seq.next_element()?.ok_or_else(|| Error::custom("short"))?;
                }
                Ok(out)
            }
        }
        d.deserialize_bytes(V)
    }
}

pub mod byte_array_64 {
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S: Serializer>(v: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = [u8; 64];
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("64 bytes")
            }
            fn visit_bytes<E: Error>(self, b: &[u8]) -> Result<Self::Value, E> {
                b.try_into().map_err(|_| E::custom("expected 64 bytes"))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = [0u8; 64];
                for slot in out.iter_mut() {
                    *slot = seq.next_element()?.ok_or_else(|| Error::custom("short"))?;
                }
                Ok(out)
            }
        }
        d.deserialize_bytes(V)
    }
}
