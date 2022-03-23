use crate::domain::{SubscriberEmail, SubscriberName};

#[derive(Debug)]
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl NewSubscriber {
    pub fn new(email: SubscriberEmail, name: SubscriberName) -> Self {
        Self { email, name }
    }
}
