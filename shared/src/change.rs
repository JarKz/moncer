use serde::Deserialize;

use crate::error::ApplicationError;

#[derive(Clone, Copy)]
pub struct NonZeroChange(i64);

impl NonZeroChange {
    pub fn into_inner(self) -> i64 {
        self.0
    }
}

impl TryFrom<i64> for NonZeroChange {
    type Error = ApplicationError;

    fn try_from(change_minor: i64) -> Result<Self, Self::Error> {
        if change_minor == 0 {
            return Err(ApplicationError::ValidationFailed {
                message: "Change must be not equal to 0.".to_owned(),
            });
        }

        Ok(Self(change_minor))
    }
}

impl<'de> Deserialize<'de> for NonZeroChange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let change_minor = i64::deserialize(deserializer)?;

        NonZeroChange::try_from(change_minor).map_err(serde::de::Error::custom)
    }
}
