use rand::distributions::{Alphanumeric, DistString};
use rand::thread_rng;

#[derive(Debug, Default)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    const TOKEN_LEN: usize = 25;
    pub fn new() -> Self {
        let token = Alphanumeric.sample_string(&mut thread_rng(), Self::TOKEN_LEN);
        Self(token)
    }

    pub fn parse(s: impl AsRef<str>) -> Result<Self, String> {
        let s = s.as_ref();
        let valid_token_len = s.len() == Self::TOKEN_LEN;
        let valid_chars = s.chars().all(char::is_alphanumeric);
        if valid_token_len && valid_chars {
            Ok(Self(s.to_string()))
        } else {
            Err(format!("{} is not a valid subscription token.", s))
        }
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::SubscriptionToken;
    use claim::assert_ok;

    #[test]
    fn generated_token_is_valid() {
        let token1 = SubscriptionToken::new();
        assert_ok!(SubscriptionToken::parse(token1.as_ref()));

        let token2 = SubscriptionToken::new();
        assert_ok!(SubscriptionToken::parse(token2.as_ref()));

        assert_ne!(token1.as_ref(), token2.as_ref());
    }
}
