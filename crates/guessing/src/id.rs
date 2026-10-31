use std::fmt;
use std::str::FromStr;

use jiff::Timestamp;
use uuid::Uuid;

/// A Player's ID: a `UUIDv7`, made from the time and random bytes the caller supplies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(Uuid);

/// A Guess's ID: a `UUIDv7`, made from the time and random bytes the caller supplies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GuessId(Uuid);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("an ID is a UUIDv7")]
pub struct MalformedId;

fn version_7(at: Timestamp, random: [u8; 10]) -> Uuid {
    // A time before 1970 has no UUIDv7; the clock never shows one.
    let millis = u64::try_from(at.as_millisecond()).unwrap_or(0);
    uuid::Builder::from_unix_timestamp_millis(millis, &random).into_uuid()
}

fn parse_version_7(text: &str) -> Result<Uuid, MalformedId> {
    let uuid = Uuid::try_parse(text).map_err(|_| MalformedId)?;
    if uuid.get_version_num() == 7 {
        Ok(uuid)
    } else {
        Err(MalformedId)
    }
}

macro_rules! uuid_v7_id {
    ($id:ident) => {
        impl $id {
            /// Makes a new ID at `at` from 10 bytes drawn from a secure random source.
            #[must_use]
            pub fn new(at: Timestamp, random: [u8; 10]) -> Self {
                Self(version_7(at, random))
            }
        }

        impl fmt::Display for $id {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(formatter)
            }
        }

        impl FromStr for $id {
            type Err = MalformedId;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                parse_version_7(text).map(Self)
            }
        }
    };
}

uuid_v7_id!(PlayerId);
uuid_v7_id!(GuessId);

#[cfg(test)]
mod tests {
    use super::*;

    const AT: Timestamp = Timestamp::constant(1_798_761_600, 0);

    #[test]
    fn an_id_is_a_uuid_v7_string_that_reads_back() {
        let id = PlayerId::new(AT, [9; 10]);
        let text = id.to_string();

        assert_eq!(text.len(), 36);
        assert_eq!(text.as_bytes()[14], b'7', "{text}");
        assert_eq!(text.parse::<PlayerId>(), Ok(id));
    }

    #[test]
    fn ids_made_from_different_random_bytes_differ() {
        assert_ne!(GuessId::new(AT, [1; 10]), GuessId::new(AT, [2; 10]));
    }

    #[test]
    fn a_text_that_is_no_uuid_v7_is_no_id() {
        for text in ["", "player", "00000000-0000-4000-8000-000000000000"] {
            assert_eq!(text.parse::<GuessId>(), Err(MalformedId), "{text:?}");
        }
    }
}
