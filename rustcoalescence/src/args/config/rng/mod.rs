use std::{fmt, marker::PhantomData};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_state::DeserializeState;

use necsim_core::cogs::{MathsCore, RngCore};
use necsim_partitioning_core::partition::PartitionSize;

mod base32;

use self::base32::Base32String;

#[derive(Debug, Serialize)]
#[serde(bound = "")]
pub enum Rng<M: MathsCore, G: RngCore<M>> {
    Seed(u64),
    Sponge(Base32String),
    State(Base32RngState<M, G>),
}

pub struct Base32RngState<M: MathsCore, G: RngCore<M>> {
    rng: G,
    marker: PhantomData<M>,
}

impl<'de, M: MathsCore, G: RngCore<M>> DeserializeState<'de, PartitionSize> for Rng<M, G> {
    fn deserialize_state<D: Deserializer<'de>>(
        partition_size: &mut PartitionSize,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let raw = RngRaw::<M, G>::deserialize(deserializer)?;

        let rng = match raw {
            RngRaw::Entropy => {
                let mut entropy = G::Seed::default();

                loop {
                    getrandom::getrandom(entropy.as_mut()).map_err(serde::de::Error::custom)?;

                    // Ensure that no protected state sponges are generated
                    if ProtectedState::from_bytes(entropy.as_mut()).is_none() {
                        break;
                    }
                }

                let sponge = Base32String::new(entropy.as_mut());

                if !partition_size.is_monolithic() {
                    return Err(serde::de::Error::custom(format!(
                        "`Entropy` rng initialisation cannot be used with partitioned \
                         simulations.\n\nTry using `Sponge({sponge})` instead."
                    )));
                }

                Self::Sponge(sponge)
            },
            RngRaw::Seed(seed) => Self::Seed(seed),
            RngRaw::Sponge(sponge) => Self::Sponge(sponge),
            RngRaw::State(state) => Self::State(state),
            RngRaw::StateElseSponge(state) => {
                match bincode::Options::deserialize(bincode::options(), &state) {
                    Ok(rng) => Self::State(Base32RngState {
                        rng,
                        marker: PhantomData::<M>,
                    }),
                    Err(_) => Self::Sponge(state),
                }
            },
        };

        Ok(rng)
    }
}

impl<M: MathsCore, G: RngCore<M>> From<G> for Base32RngState<M, G> {
    fn from(rng: G) -> Self {
        Self {
            rng,
            marker: PhantomData::<M>,
        }
    }
}

impl<M: MathsCore, G: RngCore<M>> Base32RngState<M, G> {
    #[must_use]
    pub fn into(self) -> G {
        self.rng
    }
}

impl<M: MathsCore, G: RngCore<M>> fmt::Debug for Base32RngState<M, G> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match ProtectedState::serialize(&self.rng) {
            Ok(state) => Base32String::new(&state).fmt(fmt),
            Err(_) => fmt.write_str("InvalidRngState"),
        }
    }
}

impl<M: MathsCore, G: RngCore<M>> Serialize for Base32RngState<M, G> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let state = ProtectedState::serialize(&self.rng).map_err(serde::ser::Error::custom)?;

        Base32String::new(&state).serialize(serializer)
    }
}

impl<'de, M: MathsCore, G: RngCore<M>> Deserialize<'de> for Base32RngState<M, G> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let state = Base32String::deserialize(deserializer)?;

        if let Some(state) = ProtectedState::from_bytes(&state) {
            if let Ok(rng) = ProtectedState::deserialize(state) {
                return Ok(Self {
                    rng,
                    marker: PhantomData::<M>,
                });
            }
        }

        Err(serde::de::Error::custom(format!(
            "invalid RNG state {state}"
        )))
    }
}

#[derive(Debug, Deserialize)]
#[serde(bound = "")]
#[serde(rename = "Rng")]
enum RngRaw<M: MathsCore, G: RngCore<M>> {
    Entropy,
    Seed(u64),
    #[serde(deserialize_with = "deserialize_rng_sponge")]
    Sponge(Base32String),
    State(Base32RngState<M, G>),
    #[serde(deserialize_with = "deserialize_rng_state_else_sponge")]
    StateElseSponge(Base32String),
}

fn deserialize_rng_sponge<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Base32String, D::Error> {
    let sponge = Base32String::deserialize(deserializer)?;

    if ProtectedState::from_bytes(&sponge).is_some() {
        return Err(serde::de::Error::custom(format!(
            "invalid RNG sponge but valid RNG state {sponge}\n\nDid you mean to use the `State` \
             or `StateElseSponge` variants?"
        )));
    }

    Ok(sponge)
}

fn deserialize_rng_state_else_sponge<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Base32String, D::Error> {
    let state_else_sponge = Base32String::deserialize(deserializer)?;

    if let Some(state_else_sponge) = ProtectedState::from_bytes(&state_else_sponge) {
        return Ok(Base32String::new(state_else_sponge.into_bytes()));
    }

    Err(serde::de::Error::custom(format!(
        "invalid RNG state or sponge {state_else_sponge}"
    )))
}

struct ProtectedState<'a> {
    state: &'a [u8],
}

impl<'a> ProtectedState<'a> {
    fn serialize<T: Serialize>(value: &T) -> bincode::Result<Vec<u8>> {
        let mut state = bincode::Options::serialize(bincode::options(), value)?;

        let checksum = adler::adler32_slice(&state);

        state.extend_from_slice(&checksum.to_le_bytes());

        Ok(state)
    }

    fn deserialize<T: Deserialize<'a>>(self) -> bincode::Result<T> {
        bincode::Options::deserialize(bincode::options(), self.state)
    }

    fn from_bytes(bytes: &'a [u8]) -> Option<Self> {
        let (state, checksum) = bytes.split_last_chunk()?;
        let checksum = u32::from_le_bytes(*checksum);

        if adler::adler32_slice(state) != checksum {
            return None;
        }

        Some(Self { state })
    }

    fn into_bytes(self) -> &'a [u8] {
        self.state
    }
}
