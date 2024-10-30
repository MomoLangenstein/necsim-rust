use std::{fmt, ops::Deref};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[expect(clippy::module_name_repetitions)]
#[derive(Clone)]
#[repr(transparent)]
pub struct Base32String(Box<[u8]>);

impl Base32String {
    #[must_use]
    pub fn new(bytes: &[u8]) -> Self {
        Self(bytes.to_vec().into_boxed_slice())
    }
}

impl fmt::Display for Base32String {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "{:?}",
            base32::encode(base32::Alphabet::Crockford, &self.0).to_ascii_lowercase()
        )
    }
}

impl fmt::Debug for Base32String {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "base32: {:?}",
            base32::encode(base32::Alphabet::Crockford, &self.0).to_ascii_lowercase()
        )
    }
}

impl<'de> Deserialize<'de> for Base32String {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if let Some(data) = base32::decode(
            base32::Alphabet::Crockford,
            <&str>::deserialize(deserializer)?,
        ) {
            Ok(Self(data.into_boxed_slice()))
        } else {
            Err(serde::de::Error::custom(
                "Invalid Crockford's Base32 string: must only contain alphanumeric characters.",
            ))
        }
    }
}

impl Serialize for Base32String {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        base32::encode(base32::Alphabet::Crockford, &self.0)
            .to_ascii_lowercase()
            .serialize(serializer)
    }
}

impl Deref for Base32String {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
