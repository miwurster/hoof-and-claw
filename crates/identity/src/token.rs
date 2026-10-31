use std::fmt;
use std::str::FromStr;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest as _, Sha256};

/// The opaque token in a browser's identity cookie: 32 random bytes, written in base64url.
#[derive(Clone, PartialEq, Eq)]
pub struct Token([u8; 32]);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    #[error("a token is 32 bytes in base64url")]
    Malformed,
}

/// The SHA-256 hash of a token, the only form in which the server stores it; a slow hash adds nothing for 256 random
/// bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TokenHash([u8; 32]);

impl Token {
    /// Issues a token from 32 bytes the caller drew from a secure random source.
    #[must_use]
    pub const fn from_random_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub fn hash(&self) -> TokenHash {
        TokenHash(Sha256::digest(self.0).into())
    }
}

impl TokenHash {
    /// A hash as the store reads it back.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&URL_SAFE_NO_PAD.encode(self.0))
    }
}

/// Keeps the token out of logs.
impl fmt::Debug for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Token(..)")
    }
}

impl FromStr for Token {
    type Err = TokenError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0; 32];
        match URL_SAFE_NO_PAD.decode_slice(text, &mut bytes) {
            Ok(32) => Ok(Self(bytes)),
            _ => Err(TokenError::Malformed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_reads_back_from_its_text() {
        let token = Token::from_random_bytes([7; 32]);

        assert_eq!(token.to_string().parse::<Token>(), Ok(token));
    }

    #[test]
    fn a_token_is_written_in_base64url_without_padding() {
        let text = Token::from_random_bytes([0xfb; 32]).to_string();

        assert_eq!(text.len(), 43);
        assert!(
            text.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "{text}"
        );
    }

    #[test]
    fn a_token_is_stored_as_its_sha_256_hash() {
        let token = Token::from_random_bytes([0; 32]);

        // SHA-256 of 32 zero bytes.
        assert_eq!(
            token.hash().as_bytes()[..4],
            [0x66, 0x68, 0x7a, 0xad],
            "{:x?}",
            token.hash()
        );
        assert_ne!(token.hash(), Token::from_random_bytes([1; 32]).hash());
    }

    #[test]
    fn a_text_that_is_not_32_bytes_in_base64url_is_no_token() {
        for text in [
            "",
            "short",
            "not base64url at all, but long enough to be one!",
            &"A".repeat(44),
        ] {
            assert_eq!(
                text.parse::<Token>(),
                Err(TokenError::Malformed),
                "{text:?}"
            );
        }
    }
}
