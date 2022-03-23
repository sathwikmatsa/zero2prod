use unicode_segmentation::UnicodeSegmentation;

static FORBIDDEN_CHARACTERS: [char; 14] = [
    '&', '=', ',', '"', '-', '+', '(', ')', '<', '>', '\\', '/', '{', '}',
];

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: impl AsRef<str>) -> Result<Self, String> {
        let s = s.as_ref();
        let within_256_chars = s.graphemes(true).count() <= 256;
        let has_non_whitespace = s.chars().any(|c| !c.is_whitespace());
        let no_forbidden_chars = s.chars().all(|c| !FORBIDDEN_CHARACTERS.contains(&c));

        if within_256_chars && has_non_whitespace && no_forbidden_chars {
            Ok(Self(s.to_string()))
        } else {
            Err(format!("{} is not a valid subscriber name.", s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{SubscriberName, FORBIDDEN_CHARACTERS};
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ö".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = "      ";
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "";
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for char in FORBIDDEN_CHARACTERS {
            let name = char.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
