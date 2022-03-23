use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: impl AsRef<str>) -> Result<Self, String> {
        let s = s.as_ref();
        if validate_email(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(format!("{} is not a valid subscriber email.", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use quickcheck::{Arbitrary, Gen};

    #[derive(Clone, Debug)]
    struct ValidEmailFixture(pub String);

    impl Arbitrary for ValidEmailFixture {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
